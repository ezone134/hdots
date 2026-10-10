//! The LuaJIT UI host — the migration target that replaces the `.ron` scene
//! DSL (`QUICKSHELL_MODEL.md` §Lua). Every card / surface becomes a `.lua`
//! module under `~/.config/zen-shell/ui/cards/<id>.lua` (shell surfaces later),
//! Rust stays the backend + Wayland renderer. Backend state still lives in
//! Rust: the module's `draw(ctx)` is called once per frame and paints through
//! the `ctx.ui.*` component API — the same `Cmd`s the Rust drawers emit — so
//! the exact same parity tests compare a Lua module against its Rust drawer.
//!
//! A module is preferred over `.ron` when both exist. Once a card paints via
//! Lua (parity-proven, see the images test below) its Rust fallback drawer is
//! DELETED; `.ron` remains only as the interim fallback for not-yet-converted
//! cards, then the DSL goes too. RON scenes are shadowed the moment a module
//! appears, so the two never fight.
//!
//! Security: modules run with the full Lua stdlib (LuaJIT). This is deliberate
//! for v1 — the shell launches its own config, the same trust level as the
//! RON DSL and the shell binary itself. A sandbox (`StdLib::SAFE` + no fs
//! libs) is the obvious hardening pass if configs ever become third-party.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;

use mlua::{Function, Lua, RegistryKey, Table, Value};

use crate::shell::Cmd;
use crate::ui::Pal;

/// API-building callback: given the live VM, produce the `ctx.api` table for
/// one card (backend state published for this frame).
pub(crate) type BuildApi = dyn Fn(&Lua) -> mlua::Result<Value>;

/// The context `ctx.ui.scene(...)` renders with: the renderer is Rust, but the
/// item tree is handed in by the module. `vals` is the frame's resolved
/// `SceneValues` so `{var}` placeholders and bindings resolve exactly as they
/// did when a `.ron` scene owned the card; the rest is the card box + chrome
/// flags the walk needs. Held behind an `Rc` so the `'static` Lua callback can
/// capture it without cloning per frame.
pub(crate) struct SceneRender {
    pub vals: Rc<crate::scene::SceneValues>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub hover: u32,
    pub grid: f32,
    pub show_title: bool,
    pub show_glyph: bool,
}

/// One loaded (or failed) `.lua` module, keyed by card/surface id.
struct LuaEnv {
    /// mtime the module was compiled from; `None` for a missing file. A
    /// changed mtime triggers a recompile at the next frame (hot reload).
    mtime: Option<SystemTime>,
    lua: Lua,
    /// The module's environment table + its `draw` function, kept alive in
    /// the registry so locals and functions defined at top level persist
    /// across frames.
    env: Option<(RegistryKey, RegistryKey)>,
    ok: bool,
}

impl LuaEnv {
    /// (re)load the module from `path`. `id` only names errors.
    fn load(id: &str, path: &Path) -> LuaEnv {
        let mtime = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok());
        let lua = Lua::new();
        let code = match &mtime {
            Some(_) => std::fs::read_to_string(path).unwrap_or_default(),
            None => String::new(),
        };
        let env = lua.create_table().expect("create module env");
        // The module env is a SANDBOX for writes only: reads fall through to
        // the real globals (the full LuaJIT stdlib — `tostring`, `math`,
        // `string`, `table`, …), while top-level assignments (`function draw`)
        // land in the module's own table and cannot leak between modules.
        let mt = lua.create_table().expect("create module env metatable");
        mt.set("__index", lua.globals()).expect("env metatable __index");
        env.set_metatable(Some(mt)).expect("set module env metatable");
        let chunk = lua.load(&code).set_name(id);
        let res = chunk.set_environment(env.clone()).exec();
        match res {
            Ok(_) => {
                let draw: Option<Function> = env.get("draw").ok();
                match draw {
                    Some(f) => {
                        let kd = lua
                            .create_registry_value(f)
                            .expect("registry draw fn");
                        let ke = lua.create_registry_value(env).expect("registry env");
                        LuaEnv {
                            mtime,
                            lua,
                            env: Some((ke, kd)),
                            ok: true,
                        }
                    }
                    None => {
                        eprintln!("lua[{id}]: module defines no `draw(ctx)`");
                        LuaEnv {
                            mtime,
                            lua,
                            env: None,
                            ok: false,
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("lua[{id}]: load failed: {e}");
                LuaEnv { mtime, lua, env: None, ok: false }
            }
        }
    }
}

/// Per-frame scratch the module paints into. Owned so the `ui.*` closures can
/// capture it; drained back into the caller's `Vec<Cmd>` / `region()` after
/// the module returns.
struct DrawBuffer {
    cmds: Rc<RefCell<Vec<Cmd>>>,
    hits: Rc<RefCell<Vec<(f32, f32, f32, f32, u32)>>>,
}

#[derive(Default)]
pub(crate) struct LuaHost {
    envs: HashMap<String, LuaEnv>,
}

impl LuaHost {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// True when `<id>` currently has a valid module (loading/reloading on
    /// mtime change). Used to pick Lua over `.ron` in the draw dispatch.
    pub(crate) fn has(&mut self, id: &str, path: &Path) -> bool {
        self.load(id, path)
    }

    fn load(&mut self, id: &str, path: &Path) -> bool {
        let mtime = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok());
        let fresh = match self.envs.get(id) {
            Some(e) => e.mtime != mtime && e.ok,
            None => false,
        };
        if !fresh {
            if let Some(e) = self.envs.get(id) {
                if e.mtime == mtime {
                    return e.ok;
                }
            }
        }
        let entry = LuaEnv::load(id, path);
        let ok = entry.ok;
        self.envs.insert(id.to_string(), entry);
        ok
    }

    /// Draw one card through its Lua module. Returns the hit regions the
    /// module registered (already in device px) so the caller can wire them to
    /// its own input pipeline, or `None` when no module drew (missing file,
    /// load error, no `draw`, or the module errored — in which case the scene
    /// / Rust fallback paints instead).
    ///
    /// `api` builds the per-card `ctx.api` table from backend state (see
    /// `BuildApi`). `pal` and `grid`/`hover` complete the context; all
    /// coordinates the module hands to `ctx.ui.*` are DEVICE px (base px times
    /// `ctx.s(ctx, n)` / `ctx.fs(ctx, n)` in the module).
    pub(crate) fn draw_card(
        &mut self,
        id: &str,
        path: &Path,
        v: &mut Vec<Cmd>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        hover: u32,
        pal: Pal,
        grid: f32,
        api: &BuildApi,
        scene: &SceneRender,
    ) -> Option<Vec<(f32, f32, f32, f32, u32)>> {
        if !self.load(id, path) {
            return None;
        }
        let e = self.envs.get(id)?;
        let (_ke, kd) = e.env.as_ref()?;
        let lua = &e.lua;
        let draw: Function = lua.registry_value::<Function>(kd).ok()?;
        let buf = DrawBuffer {
            cmds: Rc::new(RefCell::new(Vec::new())),
            hits: Rc::new(RefCell::new(Vec::new())),
        };

        let res = self.paint(lua, &draw, &buf, x, y, w, h, hover, pal, grid, api, scene);
        if res.is_err() {
            eprintln!("lua[{id}]: draw failed: {:?}", res.err());
            return None;
        }

        let hits = {
            let mut hh = buf.hits.borrow_mut();
            std::mem::take(&mut *hh)
        };
        let mut cc = buf.cmds.borrow_mut();
        v.append(&mut cc);
        Some(hits)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &self,
        lua: &Lua,
        draw: &Function,
        buf: &DrawBuffer,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        hover: u32,
        pal: Pal,
        grid: f32,
        api: &BuildApi,
        scene: &SceneRender,
    ) -> mlua::Result<()> {
        ctx(lua, buf, x, y, w, h, hover, pal, grid, api, scene, |ctx| {
            draw.call::<()>(ctx)
        })
    }
}

/// Build the per-frame `ctx` table and run the module's `draw` against it.
#[allow(clippy::too_many_arguments)]
fn ctx<R>(
    lua: &Lua,
    buf: &DrawBuffer,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    hover: u32,
    pal: Pal,
    grid: f32,
    api: &BuildApi,
    scene: &SceneRender,
    run: impl FnOnce(Table) -> mlua::Result<R>,
) -> mlua::Result<R> {
    let c = lua.create_table()?;
    c.set("x", x)?;
    c.set("y", y)?;
    c.set("w", w)?;
    c.set("h", h)?;
    c.set("hover", hover)?;
    c.set("grid", grid)?;
    c.set("s", lua.create_function(move |_, n: f32| Ok(n * grid))?)?;
    c.set("fs", lua.create_function(move |_, n: f32| Ok((n * grid * 10.0).round() / 10.0))?)?;

    let pal_t = lua.create_table()?;
    pal_t.set("fg", pal.fg)?;
    pal_t.set("bg", pal.bg)?;
    pal_t.set("acc", pal.acc)?;
    pal_t.set("sfg", pal.sfg)?;
    pal_t.set("fg2", crate::ui::fg2(&pal))?;
    pal_t.set("fg3", crate::ui::fg3(&pal))?;
    pal_t.set("hover", crate::ui::hover(&pal))?;
    pal_t.set("hover_hl", crate::ui::hover_hl(&pal))?;
    pal_t.set("hover_fg", crate::ui::hover_fg(&pal))?;
    pal_t.set("acc_tint", crate::ui::acc_tint(&pal))?;
    pal_t.set("ok", crate::ui::OK)?;
    pal_t.set("warn", crate::ui::WARN)?;
    pal_t.set("danger", crate::ui::DANGER)?;
    pal_t.set("info", crate::ui::INFO)?;
    pal_t.set("clear", 0u32)?;
    pal_t.set("sel_bg", crate::ui::sel_bg(&pal))?;
    pal_t.set("hairline", crate::ui::hairline(&pal))?;
    pal_t.set("hairline_hl", crate::ui::hairline_hl(&pal))?;
    pal_t.set("raised", crate::ui::raised(&pal))?;
    pal_t.set("raised_hl", crate::ui::raised_hl(&pal))?;
    c.set("pal", pal_t)?;

    c.set("ui", ui(lua, buf, pal, scene)?)?;

    let api_v = api(lua)?;
    c.set("api", api_v)?;
    run(c)
}

/// The component library a module paints with — thin wrappers over the same
/// `crate::ui` draw calls the Rust drawers use, so parity is structural.
fn ui(lua: &Lua, buf: &DrawBuffer, pal: Pal, scene: &SceneRender) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    // `ctx.ui.scene(decl)` — render a module-held declarative item tree through
    // the SAME engine that drew `.ron` scenes: the module decides the layout,
    // Rust paints it. Parses the RON decl, walks it with the frame's resolved
    // values, appends every `Cmd` and registers every hit exactly like
    // `draw_card_scene` did for a `.ron`. `anim:` is not yet supported here (a
    // module with animated fields needs the shell's table threaded through);
    // no live card uses it.
    {
        let b = buf.cmds.clone();
        let hs = buf.hits.clone();
        let x = scene.x;
        let y = scene.y;
        let w = scene.w;
        let h = scene.h;
        let hover = scene.hover;
        let grid = scene.grid;
        let show_title = scene.show_title;
        let show_glyph = scene.show_glyph;
        let vals = scene.vals.clone();
        t.set(
            "scene",
            lua.create_function(move |_, src: String| {
                let cards: crate::scene::CardScene =
                    crate::scene::parse_scene_ron(&src).map_err(|e| {
                        mlua::Error::RuntimeError(format!("ui.scene: parse: {e}"))
                    })?;
                let mut cmds: Vec<Cmd> = Vec::new();
                let (hits, _inks) = cards.draw(
                    &mut cmds,
                    x,
                    y,
                    w,
                    h,
                    crate::shell::GridScale(grid),
                    &pal,
                    &vals,
                    hover,
                    show_title,
                    show_glyph,
                );
                b.borrow_mut().append(&mut cmds);
                let mut hh = hs.borrow_mut();
                for hit in hits {
                    hh.push((hit.x, hit.y, hit.w, hit.h, hit.key));
                }
                Ok(())
            })?,
        )?;
    }
    // Every closure moves ownership of the rc handle INTO the 'static callback
    // (never borrows `buf` itself).
    {
        let b = buf.cmds.clone();
        t.set(
            "surface",
            lua.create_function(move |_, (x, y, w, h, r, color): (f32, f32, f32, f32, f32, i64)| {
                b.borrow_mut()
                    .push(Cmd::Rect { x, y, w, h, r, color: color as u32 });
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "outline",
            lua.create_function(
                move |_, (x, y, w, h, r, width, color): (f32, f32, f32, f32, f32, f32, i64)| {
                    b.borrow_mut().push(Cmd::Outline {
                        x,
                        y,
                        w,
                        h,
                        r,
                        width,
                        color: color as u32,
                    });
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the battery gauge — the shared vector primitive, so a Lua module
        // paints the exact same nub + case + water the `Battery` scene item
        // does (`push_battery_icon_parts`).
        t.set(
            "battery",
            lua.create_function(
                move |_,
                      (cx, cy, w, h, vertical, pct, line, body, fill): (
                    f32,
                    f32,
                    f32,
                    f32,
                    bool,
                    i32,
                    i64,
                    i64,
                    i64,
                )| {
                    crate::scene::push_battery_icon_parts(
                        &mut *b.borrow_mut(),
                        line as u32,
                        body as u32,
                        fill as u32,
                        pct,
                        cx,
                        cy,
                        w,
                        h,
                        vertical,
                    );
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "line",
            lua.create_function(
                move |_, (x0, y0, x1, y1, w, color): (f32, f32, f32, f32, f32, i64)| {
                    b.borrow_mut()
                        .push(Cmd::Line { x0, y0, x1, y1, w, color: color as u32 });
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the `Spark` line trace, as a shared primitive so a Lua module emits
        // the exact `Cmd::Line` sequence the scene's `SparkKind::Line` does
        // (inset None, min_w/min_h 0) — the f32 arithmetic stays in Rust, so a
        // Lua float can't drift the vertices. `sx/sy/sw/sh` are device px.
        // The optional 8th arg is the scene's `Spark` `max`: a positive value
        // divides the samples by it (the powerdraw / cpugpu pre-normalized
        // traces), omitted or non-positive falls back to the data peak — the
        // scene's own `if max > 0 { max } else { peak }` rule.
        t.set(
            "spark",
            lua.create_function(
                move |_,
                      (sx, sy, sw, sh, data, color, thickness, vmax): (
                    f32,
                    f32,
                    f32,
                    f32,
                    mlua::Table,
                    i64,
                    f32,
                    Option<f32>,
                )| {
                    let n = data.raw_len();
                    if n < 2 {
                        return Ok(());
                    }
                    let mut vals: Vec<f32> = Vec::with_capacity(n);
                    for i in 1..=n {
                        vals.push(data.get::<f32>(i)?);
                    }
                    let t = thickness;
                    let pad_v = t / 2.0 + 1.0;
                    let usable = (sh - pad_v * 2.0).max(2.0);
                    let peak = match vmax {
                        Some(m) if m > 0.0 => m,
                        _ => vals.iter().cloned().fold(0.0f32, f32::max).max(1e-6),
                    };
                    let step = sw / (vals.len() - 1) as f32;
                    let mut prev: Option<(f32, f32)> = None;
                    for (i, &d) in vals.iter().enumerate() {
                        let px = sx + i as f32 * step;
                        let py = sy + sh - pad_v - (d / peak).clamp(0.0, 1.0) * usable;
                        if let Some((qx, qy)) = prev {
                            b.borrow_mut().push(Cmd::Line {
                                x0: qx,
                                y0: qy,
                                x1: px,
                                y1: py,
                                w: t,
                                color: color as u32,
                            });
                        }
                        prev = Some((px, py));
                    }
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the `Bar` gauge, as a shared primitive — a Lua module passes the
        // already-resolved device box (sx/sy/sw/sh) plus the raw value and
        // max, and Rust reproduces the scene's exact f32 fraction + track
        // and fill rects (`vertical` fills bottom-up like the power buttons).
        t.set(
            "bar",
            lua.create_function(
                move |_,
                      (sx, sy, sw, sh, r, raw, max, fill, track, vertical): (
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    i64,
                    i64,
                    bool,
                )| {
                    let vb = &mut *b.borrow_mut();
                    vb.push(Cmd::Rect {
                        x: sx,
                        y: sy,
                        w: sw,
                        h: sh,
                        r,
                        color: track as u32,
                    });
                    let max = if max > 0.0 { max } else { 100.0 };
                    let frac = if max > 0.0 {
                        (raw / max).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    if frac > 0.001 {
                        if vertical {
                            vb.push(Cmd::Rect {
                                x: sx,
                                y: sy + sh * (1.0 - frac),
                                w: sw,
                                h: sh * frac,
                                r,
                                color: fill as u32,
                            });
                        } else {
                            vb.push(Cmd::Rect {
                                x: sx,
                                y: sy,
                                w: sw * frac,
                                h: sh,
                                r,
                                color: fill as u32,
                            });
                        }
                    }
                    Ok(())
                },
            )?,
        )?;
    }
    // text helpers — same weights as the Rust drawers (Ui/Light default,
    // title/caption step up), so a Lua module renders identically.
    t.set("text", text_fn(lua, buf, |v, x, y, t2, sz, c, ic| {
        crate::ui::text(v, x, y, t2, sz, c, ic)
    })?)?;
    t.set("textr", text_fn(lua, buf, |v, x, y, t2, sz, c, ic| {
        crate::ui::text_r(v, x, y, t2, sz, c, ic)
    })?)?;
    t.set("textc", text_fn(lua, buf, |v, x, y, t2, sz, c, ic| {
        crate::ui::text_c(v, x, y, t2, sz, c, ic)
    })?)?;
    // display-medium: the hero-numeral voice (big temp / % / clock digits)
    t.set("hero", text_fn(lua, buf, |v, x, y, t2, sz, c, _ic| {
        crate::ui::text_hero(v, x, y, t2, sz, c)
    })?)?;
    // right-anchored display-medium — the hero numeral that hugs a card's
    // right edge (the latency card's ms readout, `text_r_hero`).
    t.set("heror", text_fn(lua, buf, |v, x, y, t2, sz, c, _ic| {
        crate::ui::text_r_hero(v, x, y, t2, sz, c)
    })?)?;
    {
        let b = buf.cmds.clone();
        t.set(
            "heroc",
            lua.create_function(
                move |_, (x, y, t2, size, color, icon): (f32, f32, String, f32, i64, bool)| {
                    crate::ui::text_cw(
                        &mut *b.borrow_mut(),
                        x,
                        y,
                        t2,
                        size,
                        crate::text::Ff::Display,
                        crate::text::Fw::Medium,
                        color as u32,
                        icon,
                    );
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "title",
            lua.create_function(move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                crate::ui::title(&mut *b.borrow_mut(), x, y, t2, size, color as u32);
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // centered brand-mark glyph — the `Brand` family at the scene `Text`
        // default Light weight (the branding card's contain-fit mark).
        t.set(
            "brandc",
            lua.create_function(
                move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                    crate::ui::text_cw(
                        &mut *b.borrow_mut(),
                        x,
                        y,
                        t2,
                        size,
                        crate::text::Ff::Brand,
                        crate::text::Fw::Light,
                        color as u32,
                        false,
                    );
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // centered semibold — the cpu/mem hero percent (a top-level `Text`
        // with `center`, semibold, centered at x + w/2 of its resolved box)
        t.set(
            "titlec",
            lua.create_function(move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                crate::ui::text_cw(
                    &mut *b.borrow_mut(),
                    x,
                    y,
                    t2,
                    size,
                    crate::text::Ff::Ui,
                    crate::text::Fw::Semibold,
                    color as u32,
                    false,
                );
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "caption",
            lua.create_function(
                move |_, (x, y, t2, size, color, icon): (f32, f32, String, f32, i64, bool)| {
                    crate::ui::caption(&mut *b.borrow_mut(), x, y, t2, size, color as u32, icon);
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "captionr",
            lua.create_function(
                move |_, (x, y, t2, size, color, icon): (f32, f32, String, f32, i64, bool)| {
                    crate::ui::caption_r(&mut *b.borrow_mut(), x, y, t2, size, color as u32, icon);
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        t.set(
            "captionc",
            lua.create_function(
                move |_, (x, y, t2, size, color, icon): (f32, f32, String, f32, i64, bool)| {
                    crate::ui::caption_c(&mut *b.borrow_mut(), x, y, t2, size, color as u32, icon);
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // centered mono readout — the gauges' capacity line
        t.set(
            "monoc",
            lua.create_function(move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                crate::ui::text_c_mono(&mut *b.borrow_mut(), x, y, t2, size, color as u32);
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // right-aligned mono — the metric-row value column / header meta
        t.set(
            "monor",
            lua.create_function(move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                crate::ui::text_r_mono(&mut *b.borrow_mut(), x, y, t2, size, color as u32);
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // left-aligned mono — technical readouts (device names, sensor lines)
        t.set(
            "mono",
            lua.create_function(move |_, (x, y, t2, size, color): (f32, f32, String, f32, i64)| {
                crate::ui::text_mono(&mut *b.borrow_mut(), x, y, t2, size, color as u32);
                Ok(())
            })?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the bead gauge ring — the shared vector primitive, so a Lua module
        // paints the exact same beads the `Ring` scene item does (`bead_ring`).
        t.set(
            "ring",
            lua.create_function(
                move |_,
                      (cx, cy, radius, dot_d, beads, lit, col, dim): (
                    f32,
                    f32,
                    f32,
                    f32,
                    i64,
                    i64,
                    i64,
                    i64,
                )| {
                    crate::scene::bead_ring(
                        &mut *b.borrow_mut(),
                        cx,
                        cy,
                        radius,
                        dot_d,
                        beads.max(0) as usize,
                        lit.max(0) as usize,
                        col as u32,
                        dim as u32,
                    );
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the lunar disc — the shared vector primitive, so a Lua module paints
        // the exact same dark base disc + lit terminator slivers the scene's
        // `Moon` item does (`moon_disc`). `phase` is the 0..1 cycle fraction.
        // The scene draws the primitive at the ORIGIN and translates the rects
        // (its `dark`/`lit`/`step` overrides repaint and re-run the sliver
        // loop there), so this binding reproduces that same translate step —
        // the f32 sums land bit-identically.
        t.set(
            "moon",
            lua.create_function(
                move |_, (cx, cy, d, phase): (f32, f32, f32, f64)| {
                    let mut sub: Vec<Cmd> = Vec::new();
                    crate::ui::moon_disc(&mut sub, 0.0, 0.0, d, phase, &pal);
                    for cmd in sub {
                        match cmd {
                            Cmd::Rect { x: rx, y: ry, w, h, r, color } => {
                                b.borrow_mut().push(Cmd::Rect { x: cx + rx, y: cy + ry, w, h, r, color });
                            }
                            other => b.borrow_mut().push(other),
                        }
                    }
                    Ok(())
                },
            )?,
        )?;
    }
    {
        let b = buf.cmds.clone();
        // the `Spectrum` bar grid, as a shared primitive so a Lua module emits
        // the exact `Cmd` sequence the scene's `Spectrum` item does — the f32
        // bar arithmetic stays in Rust, so a Lua double can't drift the
        // vertices. `sx/sy/sw/sh` are device px; `kind` is
        // "rounded"/"block"/"wave"; `inset`/`min_bar_w`/`thickness`/`block_r`
        // are already device px; `floor` is the raw cull threshold.
        t.set(
            "spectrum",
            lua.create_function(
                move |_,
                      (
                    sx,
                    sy,
                    sw,
                    sh,
                    data,
                    kind,
                    floor,
                    bar_frac,
                    inset,
                    min_bar_w,
                    thickness,
                    block_r,
                    base,
                    accent_every,
                    tint,
                ): (
                    f32,
                    f32,
                    f32,
                    f32,
                    mlua::Table,
                    String,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    f32,
                    i64,
                    i64,
                    f32,
                )| {
                    let n = data.raw_len();
                    if n == 0 {
                        return Ok(());
                    }
                    let mut vals: Vec<f32> = Vec::with_capacity(n);
                    for i in 1..=n {
                        vals.push(data.get::<f32>(i)?);
                    }
                    let kind = match kind.as_str() {
                        "wave" => crate::scene::SpectrumKind::Wave,
                        "block" => crate::scene::SpectrumKind::Block,
                        _ => crate::scene::SpectrumKind::Rounded,
                    };
                    crate::scene::push_spectrum(
                        &mut *b.borrow_mut(),
                        sx,
                        sy,
                        sw,
                        sh,
                        &vals,
                        kind,
                        floor,
                        bar_frac,
                        inset,
                        min_bar_w,
                        thickness,
                        block_r,
                        base as u32,
                        accent_every.max(0) as u32,
                        tint,
                        pal.fg,
                    );
                    Ok(())
                },
            )?,
        )?;
    }
    // mix two 0xRRGGBBAA colors toward each other.
    t.set(
        "mix",
        lua.create_function(move |_, (a, b, k): (i64, i64, f64)| Ok(crate::ui::mix(a as u32, b as u32, k as f32)))?,
    )?;
    // char-aware truncation — the scene's `chars().take(n)` (a byte cap would
    // split a multi-byte glyph). `len` is its `chars().count()` companion.
    t.set(
        "sub",
        lua.create_function(move |_, (s, n): (String, i64)| {
            Ok(s.chars().take(n.max(0) as usize).collect::<String>())
        })?,
    )?;
    t.set(
        "len",
        lua.create_function(move |_, s: String| Ok(s.chars().count() as i64))?,
    )?;
    {
        let b = buf.hits.clone();
        t.set(
            "hit",
            lua.create_function(move |_, (x, y, w, h, key): (f32, f32, f32, f32, i64)| {
                b.borrow_mut().push((x, y, w, h, key as u32));
                Ok(())
            })?,
        )?;
    }
    Ok(t)
}

fn text_fn(
    lua: &Lua,
    buf: &DrawBuffer,
    f: impl Fn(&mut Vec<Cmd>, f32, f32, String, f32, u32, bool) + 'static,
) -> mlua::Result<Function> {
    let b = buf.cmds.clone();
    lua.create_function(move |_, (x, y, t, size, color, icon): (f32, f32, String, f32, i64, bool)| {
        f(&mut *b.borrow_mut(), x, y, t, size, color as u32, icon);
        Ok(())
    })
}

/// Build the `ctx.api` table the notes module reads — backend state published
/// to the Lua layer each frame. Pure: takes the state snapshot by value so the
/// shell borrow is released before the host runs. (Per-card api serializers
/// live here for now; once more cards migrate they can move to their module.)
pub(crate) fn build_notes_lua_api(
    lua: &Lua,
    notes: &[(String, String)],
    title: &str,
    body: &str,
    input: Option<u8>,
    scroll: usize,
    scene_owns_header: bool,
    scene_owns_rows: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    let lists = lua.create_table()?;
    for (i, (t, b)) in notes.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("title", t.as_str())?;
        row.set("body", b.as_str())?;
        lists.set(i + 1, row)?;
    }
    api.set("lists", lists)?;
    api.set("count", notes.len() as i64)?;
    match input {
        Some(0) => api.set("input", 0)?,
        Some(1) => api.set("input", 1)?,
        _ => api.set("input", Value::Nil)?,
    }
    api.set("title", title)?;
    api.set("body", body)?;
    api.set("scroll", scroll as i64)?;
    api.set("scene_owns_header", scene_owns_header)?;
    api.set("scene_owns_rows", scene_owns_rows)?;
    let keys = lua.create_table()?;
    keys.set("input", crate::shell::NOTES_KEY_INPUT as i64)?;
    keys.set("save", crate::shell::NOTES_KEY_SAVE as i64)?;
    api.set("keys", keys)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the todo module reads — the To-Do card's whole
/// backend state, published to Lua each frame. Pure: everything is taken by
/// value (or snapshot) so the shell borrow is released before the host runs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_todo_lua_api(
    lua: &Lua,
    todos: &[(String, bool)],
    scroll: usize,
    input: Option<&str>,
    scene_owns_header: bool,
    scene_owns_rows: bool,
    scene_owns_composer: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    let list = lua.create_table()?;
    let mut done = 0i64;
    for (i, (text, is_done)) in todos.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("text", text.as_str())?;
        row.set("done", *is_done)?;
        if *is_done {
            done += 1;
        }
        list.set(i + 1, row)?;
    }
    api.set("todos", list)?;
    api.set("count", todos.len() as i64)?;
    api.set("done", done)?;
    api.set("scroll", scroll as i64)?;
    match input {
        Some(s) => api.set("input", s)?,
        None => api.set("input", Value::Nil)?,
    }
    api.set("scene_owns_header", scene_owns_header)?;
    api.set("scene_owns_rows", scene_owns_rows)?;
    api.set("scene_owns_composer", scene_owns_composer)?;
    api.set("card_show_title", card_show_title)?;
    let keys = lua.create_table()?;
    keys.set("toggle", crate::shell::TODO_KEY_TOGGLE as i64)?;
    keys.set("delete", crate::shell::TODO_KEY_DELETE as i64)?;
    keys.set("input", crate::shell::TODO_KEY_INPUT as i64)?;
    keys.set("add", crate::shell::TODO_KEY_ADD as i64)?;
    api.set("keys", keys)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the clock module reads — the hero time and the
/// date line, the Clock card's only two values. Pure: both strings are taken
/// by value so the shell borrow is released before the host runs.
pub(crate) fn build_clock_lua_api(lua: &Lua, clock: &str, date: &str) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("clock", clock)?;
    api.set("date", date)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the cpu module reads — the load/usage card's
/// whole backend state (the gauge ink ladder, the header meta, the three
/// metric rows). `cpu_lit` is pre-resolved in f32 so the Lua bead count can't
/// drift off the scene's. Pure: everything by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_cpu_lua_api(
    lua: &Lua,
    cpu: i32,
    cpu_meta: &str,
    cpu_gauge_col: u32,
    cpu_lit: i32,
    rows: &[(String, String, u32)],
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("cpu", cpu)?;
    api.set("cpu_meta", cpu_meta)?;
    api.set("cpu_gauge_col", cpu_gauge_col)?;
    api.set("cpu_lit", cpu_lit)?;
    let list = lua.create_table()?;
    for (i, (label, value, color)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("value", value.as_str())?;
        row.set("color", *color)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the mem module reads — the memory card's whole
/// backend state (usage %, the four metric rows, the swap bar). `mem_lit` is
/// pre-resolved in f32 so the Lua bead count can't drift off the scene's.
/// Pure: everything by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_mem_lua_api(
    lua: &Lua,
    mem_pct: i32,
    mem_lit: i32,
    rows: &[(String, String)],
    swap_pct: i32,
    swap_show: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("mem", mem_pct)?;
    api.set("mem_lit", mem_lit)?;
    let list = lua.create_table()?;
    for (i, (label, value)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("value", value.as_str())?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    api.set("swap_pct", swap_pct)?;
    api.set("swap_show", swap_show)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the disk module reads — the mount list's whole
/// backend state (the header meta, the per-mount label/value/percent rows with
/// their pre-resolved warn/crit ink, and the live/empty gate). Pure: everything
/// is taken by value so the shell borrow is released before the host runs.
pub(crate) fn build_disk_lua_api(
    lua: &Lua,
    meta: &str,
    rows: &[(String, String, i32, u32)],
    live: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("disk_meta", meta)?;
    let list = lua.create_table()?;
    for (i, (label, value, pct, color)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("value", value.as_str())?;
        row.set("pct", *pct)?;
        row.set("color", *color)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    api.set("disk_live", live)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the kblayout module reads — the one live value
/// (the resolved layout label; "Unknown" when no keyboard reports one). Pure:
/// by value.
pub(crate) fn build_kblayout_lua_api(lua: &Lua, kb_layout: &str) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set(
        "kb_layout",
        if kb_layout.is_empty() { "Unknown" } else { kb_layout },
    )?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the ticker module reads — the Prices card's whole
/// backend state (the header meta, the status line, the crypto rows with their
/// pre-formatted chg% and resolved ok/danger ink, the scroll window and the
/// row-key base) plus the header toggles. Pure: everything by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_ticker_lua_api(
    lua: &Lua,
    meta: &str,
    status: &str,
    rows: &[(String, String, String, u32)],
    scroll: usize,
    key_base: u32,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("meta", meta)?;
    api.set("ticker_status", status)?;
    api.set("scroll", scroll as i64)?;
    api.set("key_base", key_base as i64)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (sym, price, chg, chg_color)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("sym", sym.as_str())?;
        row.set("price", price.as_str())?;
        row.set("chg", chg.as_str())?;
        row.set("chg_color", *chg_color)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the activewin module reads — the twin-state hero
/// (empty vs live app class/title, the 28-char cap and the per-app RSS pill).
/// Mirrors the `scene_values` activewin bindings. Pure: by value.
pub(crate) fn build_activewin_lua_api(
    lua: &Lua,
    class: &str,
    title: &str,
    rss: &str,
    has_rss: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("active_win_empty", class.is_empty())?;
    api.set("active_win_has", !class.is_empty())?;
    api.set("aw_class", class)?;
    api.set("aw_title", title)?;
    api.set("aw_has_rss", has_rss)?;
    api.set("aw_rss", rss)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the pkgupdates module reads — the hero count and
/// its mutually exclusive pending/ok gates. Mirrors the `scene_values`
/// pkgupdates bindings. Pure: by value.
pub(crate) fn build_pkgupdates_lua_api(lua: &Lua, count: i32) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("pkg_pending", count > 0)?;
    api.set("pkg_ok", count == 0)?;
    api.set("pkg_count", count.to_string())?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the sensors module reads — the per-sensor gates
/// plus the three pre-formatted value strings, computed with the exact same
/// octant/format ladders as `scene_values`. Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sensors_lua_api(
    lua: &Lua,
    has_accel: bool,
    pitch: f32,
    roll: f32,
    heading: Option<f32>,
    gyro: Option<(f32, f32, f32)>,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    let none = !has_accel && heading.is_none() && gyro.is_none();
    api.set("sensors_none", none)?;
    api.set("sensors_tilt", !none)?;
    api.set("sensors_compass", heading.is_some())?;
    api.set("sensors_gyro", gyro.is_some())?;
    api.set("sensor_tilt", format!("pitch {:.0}°  roll {:.0}°", pitch, roll))?;
    api.set(
        "sensor_compass",
        match heading {
            Some(hdg) => {
                let arrow = match (0.5 + hdg / 360.0 * 8.0) as usize % 8 {
                    0 => crate::icons::ICON_GAUGE_0,
                    1 => crate::icons::ICON_GAUGE_1,
                    2 => crate::icons::ICON_GAUGE_2,
                    3 => crate::icons::ICON_GAUGE_3,
                    4 => crate::icons::ICON_GAUGE_4,
                    5 => crate::icons::ICON_GAUGE_5,
                    6 => crate::icons::ICON_GAUGE_6,
                    _ => crate::icons::ICON_GAUGE_7,
                };
                format!("{arrow} {:.0}°", hdg)
            }
            None => String::new(),
        },
    )?;
    api.set(
        "sensor_gyro",
        match gyro {
            Some((gx, gy, gz)) => format!("{gx:3.0} {gy:3.0} {gz:3.0}"),
            None => String::new(),
        },
    )?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the sshvpn module reads — the status line + one
/// row per connection (globe glyph for VPN lines, shield for SSH). Mirrors the
/// `scene_values` sshvpn bindings. Pure: by value.
pub(crate) fn build_sshvpn_lua_api(
    lua: &Lua,
    lines: &[String],
    status: &str,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("sshvpn_status", status)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, line) in lines.iter().enumerate() {
        let row = lua.create_table()?;
        row.set(
            "icon",
            if line.starts_with("VPN") {
                crate::icons::ICON_GLOBE
            } else {
                crate::icons::ICON_SHIELD
            },
        )?;
        row.set("line", line.as_str())?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the conninfo module reads — the "{n} addrs" meta,
/// the list gate, and one row per metric (label + right mono value, each with
/// its already-resolved ink: fg3 for gateway/dns, fg otherwise). Mirrors the
/// `scene_values` `conn_rows_v` / `conn_meta` / `conn_empty` bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_conninfo_lua_api(
    lua: &Lua,
    meta: &str,
    rows: &[(String, String, u32, u32)],
    live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("conn_meta", meta)?;
    api.set("conn_live", live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (label, value, lcolor, vcolor)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("value", value.as_str())?;
        row.set("lcolor", *lcolor)?;
        row.set("vcolor", *vcolor)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the journaltail module reads — the "{n} lines"
/// meta, the list gate, and one row per log line (text + already-resolved
/// severity ink). Mirrors the `scene_values` `jr_tail_rows` / `jr_meta` /
/// `jr_empty` bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_journaltail_lua_api(
    lua: &Lua,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("jr_meta", meta)?;
    api.set("jr_live", live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (text, color)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("text", text.as_str())?;
        row.set("color", *color)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the systemdunits module reads — the state glyph
/// (check / shield), the "{n} failed" meta, the list gate, and one row per
/// failed unit (name + the shared danger ink and the trailing ✕ glyph).
/// Mirrors the `scene_values` `sus_rows` / `sus_meta` / `sus_icon` bindings.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_systemdunits_lua_api(
    lua: &Lua,
    icon: &str,
    meta: &str,
    rows: &[(String, u32)],
    live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("sus_icon", icon)?;
    api.set("sus_meta", meta)?;
    api.set("sus_live", live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let close = crate::icons::ICON_CLOSE;
    let list = lua.create_table()?;
    for (i, (unit, color)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("unit", unit.as_str())?;
        row.set("color", *color)?;
        row.set("icon", close)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the smarthealth module reads — the state glyph
/// (check / shield), the "{n} disk(s)" meta, the list gate, and one row per
/// disk: mono device name, dim model, right mono temp and the ✓/✕ status
/// glyph, each with its already-resolved ink. Mirrors the `scene_values`
/// `sm_rows` / `sm_meta` / `sm_icon` bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_smarthealth_lua_api(
    lua: &Lua,
    icon: &str,
    meta: &str,
    rows: &[(String, String, String, u32, u32, u32, u32, String)],
    live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("sm_icon", icon)?;
    api.set("sm_meta", meta)?;
    api.set("sm_live", live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (dev, model, temp, dc, mc, tc, sc, sg)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("dev", dev.as_str())?;
        row.set("model", model.as_str())?;
        row.set("temp", temp.as_str())?;
        row.set("dcolor", *dc)?;
        row.set("mcolor", *mc)?;
        row.set("tcolor", *tc)?;
        row.set("scolor", *sc)?;
        row.set("sg", sg.as_str())?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the docker module reads — the empty gate and one
/// row per container: a status dot (already-resolved ink), the name and the
/// (uncapped) image label. Mirrors the `scene_values` `docker_rows` /
/// `docker_empty` bindings. Pure.
pub(crate) fn build_docker_lua_api(
    lua: &Lua,
    rows: &[(u32, String, String)],
    empty: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("docker_empty", empty)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (dot, name, image)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("dot", *dot)?;
        row.set("name", name.as_str())?;
        row.set("image", image.as_str())?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the fans module reads — the live "{n} fans"
/// meta, the empty/live gates, the header glyph, and one row per hwmon fan:
/// the dim label, the right mono rpm and the meter fraction (rpm / decayed
/// peak) with its resolved inks. Mirrors the `scene_values` `fans_rows` /
/// `fan_meta` / `fans_empty` bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_fans_lua_api(
    lua: &Lua,
    glyph: &str,
    meta: &str,
    rows: &[(String, String, f32, u32, u32)],
    fans_empty: bool,
    fans_live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("glyph", glyph)?;
    api.set("meta", meta)?;
    api.set("fans_empty", fans_empty)?;
    api.set("fans_live", fans_live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (label, rpm, frac, lcolor, rcolor)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("rpm", rpm.as_str())?;
        row.set("frac", *frac)?;
        row.set("lcolor", *lcolor)?;
        row.set("rcolor", *rcolor)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the procmon module reads — the live "CPU {n}%"
/// meta and one row per top process (name + fraction of the top process's
/// ticks). Mirrors the `scene_values` `top_procs_rows` binding. Pure.
pub(crate) fn build_procmon_lua_api(
    lua: &Lua,
    meta: &str,
    rows: &[(String, f32)],
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("meta", meta)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (name, frac)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("name", name.as_str())?;
        row.set("frac", *frac)?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the diskio module reads — the two normalized
/// history traces (read / write, already divided by the deque peak exactly as
/// `scene_values`' `disk_r_spark` / `disk_w_spark` are). Pure: by value.
pub(crate) fn build_diskio_lua_api(
    lua: &Lua,
    r_data: &[f32],
    w_data: &[f32],
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("card_show_title", card_show_title)?;
    let r = lua.create_table()?;
    for (i, v) in r_data.iter().enumerate() {
        r.set(i + 1, *v)?;
    }
    api.set("r_data", r)?;
    let w = lua.create_table()?;
    for (i, v) in w_data.iter().enumerate() {
        w.set(i + 1, *v)?;
    }
    api.set("w_data", w)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the network module reads — the y-axis caption
/// (bytes→bits/s ladder, empty until two samples), the live down/up speed
/// strings, and the two pre-normalized history traces (download / upload,
/// already divided by the deque peak like `scene_values`' bindings). Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_network_lua_api(
    lua: &Lua,
    meta: &str,
    net_down: &str,
    net_up: &str,
    down_data: &[f32],
    up_data: &[f32],
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("meta", meta)?;
    api.set("net_down", net_down)?;
    api.set("net_up", net_up)?;
    api.set("card_show_title", card_show_title)?;
    let down = lua.create_table()?;
    for (i, v) in down_data.iter().enumerate() {
        down.set(i + 1, *v)?;
    }
    api.set("down_data", down)?;
    let up = lua.create_table()?;
    for (i, v) in up_data.iter().enumerate() {
        up.set(i + 1, *v)?;
    }
    api.set("up_data", up)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the powerdraw module reads — the two live watt
/// strings (battery `pw_b` / GPU `pw_g`), the two pre-normalized history
/// traces (already divided by the deque peak exactly as `scene_values`'
/// `pw_b_series` / `pw_g_series` are), and the live / idle / empty gates plus
/// the header toggles. The series tables are published only once a trace
/// exists (the scene's conditional binding), so the module's `if
/// api.pw_b_series` mirrors the `Spark` item's own `len >= 2` guard. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_powerdraw_lua_api(
    lua: &Lua,
    pw_b: &str,
    pw_g: &str,
    b_data: &[f32],
    g_data: &[f32],
    live: bool,
    idle: bool,
    empty: bool,
    card_show_title: bool,
    card_show_glyph: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("pw_b", pw_b)?;
    api.set("pw_g", pw_g)?;
    api.set("pw_live", live)?;
    api.set("pw_idle", idle)?;
    api.set("pw_empty", empty)?;
    api.set("card_show_title", card_show_title)?;
    api.set("card_show_glyph", card_show_glyph)?;
    if b_data.len() >= 2 {
        let t = lua.create_table()?;
        for (i, v) in b_data.iter().enumerate() {
            t.set(i + 1, *v)?;
        }
        api.set("pw_b_series", t)?;
    }
    if g_data.len() >= 2 {
        let t = lua.create_table()?;
        for (i, v) in g_data.iter().enumerate() {
            t.set(i + 1, *v)?;
        }
        api.set("pw_g_series", t)?;
    }
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the cpugpu module reads — the right-anchored
/// "CPU {n}%" / "GPU {n}%" legends (the GPU's already "GPU n/a" with the same
/// right anchor when no source exists), the colour-dot inks (the mode-aware
/// `mix(...)` pair the drawer used: brighter in dark, darker in light), the
/// 0..1 CPU/GPU histories the two trace lines draw, and the live gate that
/// hides the GPU trace + dot while no source exists (`self.gpu >= 0` — the
/// history otherwise fills with zeros, and a flat line at the plot's floor is
/// just noise). Mirrors the `cpugpu_*` / `cpu_spark` / `gpu_spark` bindings.
/// Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_cpugpu_lua_api(
    lua: &Lua,
    cpu_txt: &str,
    gpu_txt: &str,
    cpu_ink: u32,
    gpu_ink: u32,
    cpu_data: &[f32],
    gpu_data: &[f32],
    gpu_live: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("cpu_txt", cpu_txt)?;
    api.set("gpu_txt", gpu_txt)?;
    api.set("cpu_ink", cpu_ink)?;
    api.set("gpu_ink", gpu_ink)?;
    api.set("gpu_live", gpu_live)?;
    if cpu_data.len() >= 2 {
        let t = lua.create_table()?;
        for (i, v) in cpu_data.iter().enumerate() {
            t.set(i + 1, *v)?;
        }
        api.set("cpu_data", t)?;
    }
    if gpu_live && gpu_data.len() >= 2 {
        let t = lua.create_table()?;
        for (i, v) in gpu_data.iter().enumerate() {
            t.set(i + 1, *v)?;
        }
        api.set("gpu_data", t)?;
    }
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the latency module reads — the header glyph +
/// probe-host meta, the peak-normalized 0..1 probe series (empty until two
/// probes land), and the drawer's own readout: the hero ms value + ink, the
/// status word and the idle gate. Mirrors the `lat_*` scene bindings, so the
/// drawer's 120 ms peak floor and OK/WARN/DANGER thresholds stay in Rust.
/// `rerun_glyph` is the re-probe button's refresh glyph. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_latency_lua_api(
    lua: &Lua,
    idle_ink: u32,
    glyph: &str,
    meta: &str,
    rerun_glyph: &str,
    lat_state: u32,
    lat_current: u32,
    lat_history: &[u32],
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("glyph", glyph)?;
    api.set("meta", meta)?;
    api.set("rerun_glyph", rerun_glyph)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let (val, ink) = match lat_state {
        0 => ("--".to_string(), None),
        1 => ("…".to_string(), None),
        3 => ("ERR".to_string(), Some(crate::ui::DANGER)),
        _ => {
            let c = if lat_current < 60 {
                Some(crate::ui::OK)
            } else if lat_current < 120 {
                Some(crate::ui::WARN)
            } else {
                Some(crate::ui::DANGER)
            };
            (format!("{lat_current}"), c)
        }
    };
    api.set("val", val)?;
    api.set(
        "val_ink",
        match ink {
            Some(c) => c,
            None => idle_ink,
        },
    )?;
    let status = match lat_state {
        0 => "idle",
        1 => "probing…",
        3 => "no reply",
        _ => {
            if lat_current < 60 {
                "smooth"
            } else if lat_current < 120 {
                "slow"
            } else {
                "bad"
            }
        }
    };
    api.set("status", status)?;
    let series: Vec<f32> = lat_history.iter().map(|&ms| ms as f32).collect();
    api.set("idle", series.len() < 2)?;
    if series.len() >= 2 {
        let peak = series.iter().fold(120.0_f32, |a, &b| a.max(b)).max(1.0);
        let t = lua.create_table()?;
        for (i, v) in series.iter().enumerate() {
            t.set(i + 1, (v / peak).clamp(0.0, 1.0))?;
        }
        api.set("series", t)?;
    }
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the thermal module reads — the "{n} zones"
/// meta, the header glyph, the live/empty gates, and one chip per zone: the
/// zone caption, the "{temp}°" sub line (already at its warn/crit ink) and
/// the resting hover-well fill. Mirrors the `thermal_cells` bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_thermal_lua_api(
    lua: &Lua,
    glyph: &str,
    meta: &str,
    cells: &[(String, String, u32)],
    thermal_empty: bool,
    thermal_live: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("glyph", glyph)?;
    api.set("meta", meta)?;
    api.set("thermal_empty", thermal_empty)?;
    api.set("thermal_live", thermal_live)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    let list = lua.create_table()?;
    for (i, (label, sub, sub_color)) in cells.iter().enumerate() {
        let cell = lua.create_table()?;
        cell.set("label", label.as_str())?;
        cell.set("sub", sub.as_str())?;
        cell.set("sub_color", *sub_color)?;
        list.set(i + 1, cell)?;
    }
    api.set("cells", list)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the micmeter module reads — the header glyph
/// and "{n}%" meta, the 0..1 level (0 while muted), the bar's resolved fill
/// ink, the live status line with its ink, and the mute button's hover-aware
/// fill, glyph+label and ink. Mirrors the `mic_*` scene bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_micmeter_lua_api(
    lua: &Lua,
    icon: &str,
    meta: &str,
    lvl: f32,
    bar_ink: u32,
    status: &str,
    status_ink: u32,
    btn_fill: u32,
    btn_label: &str,
    btn_ink: u32,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("icon", icon)?;
    api.set("meta", meta)?;
    api.set("lvl", lvl)?;
    api.set("bar_ink", bar_ink)?;
    api.set("status", status)?;
    api.set("status_ink", status_ink)?;
    api.set("btn_fill", btn_fill)?;
    api.set("btn_label", btn_label)?;
    api.set("btn_ink", btn_ink)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the moon module reads — the header glyph and the
/// "{:.0}%" illumination meta, the 0..1 cycle fraction the disc paints, the
/// phase name / "{:.0}% lit" / days-to-full caption strings, and the header
/// gates. One frame's clock sample feeds the lot. Mirrors the `moon_*` scene
/// bindings. Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_moon_lua_api(
    lua: &Lua,
    glyph: &str,
    meta: &str,
    phase: f64,
    name: &str,
    lit: &str,
    sub: &str,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("glyph", glyph)?;
    api.set("meta", meta)?;
    api.set("phase", phase)?;
    api.set("name", name)?;
    api.set("lit", lit)?;
    api.set("sub", sub)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the viz module reads — the visualizer's live
/// series (already clamped to the card's `viz.len().min(24)` and zero-padded so
/// the bar pitch holds), the active style, the silent flag, the style button's
/// key and the header gates. Mirrors the `viz_*` scene bindings. Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_viz_lua_api(
    lua: &Lua,
    series: &[f32],
    style: u8,
    silent: bool,
    key: u32,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    let data = lua.create_table()?;
    for (i, v) in series.iter().enumerate() {
        data.set(i + 1, *v)?;
    }
    api.set("series", data)?;
    api.set("style", style as i64)?;
    api.set("silent", silent)?;
    api.set("key", key as i64)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the branding module reads — the contain-fit mark
/// (the glyph plus its per-frame BASE-px size and top, exactly the scene's
/// `brand_size` / `brand_y` bindings), the empty flag, and the header gates.
/// Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_branding_lua_api(
    lua: &Lua,
    glyph: &str,
    size: f32,
    y: f32,
    live: bool,
    empty: bool,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("glyph", glyph)?;
    api.set("size", size)?;
    api.set("y", y)?;
    api.set("live", live)?;
    api.set("empty", empty)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the GPU module reads — the util hero/bar, the
/// temp row, the VRAM row/bar (absent when the source exposes no total), and
/// the empty/live gates. Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_gpu_lua_api(
    lua: &Lua,
    live: bool,
    empty: bool,
    pct: i32,
    frac: &str,
    temp: &str,
    vram_live: bool,
    vram_txt: &str,
    vram_frac: &str,
    card_show_glyph: bool,
    card_show_title: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("live", live)?;
    api.set("empty", empty)?;
    api.set("pct", format!("{pct}%"))?;
    api.set("frac", frac)?;
    api.set("temp", temp)?;
    api.set("vram_live", vram_live)?;
    api.set("vram_txt", vram_txt)?;
    api.set("vram_frac", vram_frac)?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the powerh module reads — the five button fills,
/// inks, hold fractions and their sliver gates, plus the shared liquid ink.
/// Pure: by value. `bg`/`ink` are already resolved (hover, holding and the
/// danger ladder folded in), so the module paints without a `pal` lookup.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_powerh_lua_api(
    lua: &Lua,
    card_show_glyph: bool,
    card_show_title: bool,
    hold_fill: u32,
    bg: &[u32; 5],
    ink: &[u32; 5],
    hold: &[String; 5],
    hold_on: &[bool; 5],
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("card_show_glyph", card_show_glyph)?;
    api.set("card_show_title", card_show_title)?;
    api.set("hold_fill", hold_fill)?;
    let btns = lua.create_table()?;
    for i in 0..5 {
        let b = lua.create_table()?;
        b.set("bg", bg[i])?;
        b.set("ink", ink[i])?;
        b.set("hold", hold[i].clone())?;
        b.set("on", hold_on[i])?;
        btns.set(i + 1, b)?;
    }
    api.set("buttons", btns)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the water module reads — the hydration card's
/// whole backend state (the ring ink, hero/glass readout, goal caption, the
/// two edit-button inks and the header meta). `water_lit` is pre-resolved in
/// f32 so the Lua bead count can't drift off the scene's. Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_water_lua_api(
    lua: &Lua,
    water_lit: i32,
    water_ring_ink: u32,
    water_hero: &str,
    water_goal_txt: &str,
    water_hero_ink: u32,
    water_dim: u32,
    water_meta: &str,
    water_dec_bg: u32,
    water_inc_bg: u32,
    card_show_title: bool,
    card_show_glyph: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("water_lit", water_lit)?;
    api.set("water_ring_ink", water_ring_ink)?;
    api.set("water_hero", water_hero)?;
    api.set("water_goal_txt", water_goal_txt)?;
    api.set("water_hero_ink", water_hero_ink)?;
    api.set("water_dim", water_dim)?;
    api.set("water_meta", water_meta)?;
    api.set("water_dec_bg", water_dec_bg)?;
    api.set("water_inc_bg", water_inc_bg)?;
    api.set("card_show_title", card_show_title)?;
    api.set("card_show_glyph", card_show_glyph)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the eye-rest module reads — the focus/rest bead
/// ring's live fraction, the mm:ss hero + phase word, the toggle label, and the
/// drawer's 3-state edit-button fills (already hover-aware). Pure: by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_eyerest_lua_api(
    lua: &Lua,
    er_meta: &str,
    er_hero: &str,
    er_state: &str,
    er_tog_lbl: &str,
    er_lit: i32,
    er_ring_ink: u32,
    er_hero_ink: u32,
    er_tog_bg: u32,
    er_rst_bg: u32,
    card_show_title: bool,
    card_show_glyph: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("er_meta", er_meta)?;
    api.set("er_hero", er_hero)?;
    api.set("er_state", er_state)?;
    api.set("er_tog_lbl", er_tog_lbl)?;
    api.set("er_lit", er_lit)?;
    api.set("er_ring_ink", er_ring_ink)?;
    api.set("er_hero_ink", er_hero_ink)?;
    api.set("er_tog_bg", er_tog_bg)?;
    api.set("er_rst_bg", er_rst_bg)?;
    api.set("card_show_title", card_show_title)?;
    api.set("card_show_glyph", card_show_glyph)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the pomodoro module reads — the countdown hero +
/// phase word + bar fraction, the session meta, the idle steppers' centered
/// readout offsets, and the drawer's 3-state edit-button fills (already
/// hover-aware). Pure: everything by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_pomodoro_lua_api(
    lua: &Lua,
    pomo_meta: &str,
    pomo_hero: &str,
    pomo_ink: u32,
    pomo_bar_ink: u32,
    pomo_phase_lbl: &str,
    pomo_frac: &str,
    pomo_idle: bool,
    pomo_focus_readout: &str,
    pomo_dec_dx: f32,
    pomo_inc_dx: f32,
    pomo_chip_fill: u32,
    pomo_tog_fill: u32,
    pomo_tog_hover: u32,
    pomo_tog_lbl: &str,
    card_show_title: bool,
    card_show_glyph: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("pomo_meta", pomo_meta)?;
    api.set("pomo_hero", pomo_hero)?;
    api.set("pomo_ink", pomo_ink)?;
    api.set("pomo_bar_ink", pomo_bar_ink)?;
    api.set("pomo_phase_lbl", pomo_phase_lbl)?;
    api.set("pomo_frac", pomo_frac)?;
    api.set("pomo_idle", pomo_idle)?;
    api.set("pomo_focus_readout", pomo_focus_readout)?;
    api.set("pomo_dec_dx", pomo_dec_dx)?;
    api.set("pomo_inc_dx", pomo_inc_dx)?;
    api.set("pomo_chip_fill", pomo_chip_fill)?;
    api.set("pomo_tog_fill", pomo_tog_fill)?;
    api.set("pomo_tog_hover", pomo_tog_hover)?;
    api.set("pomo_tog_lbl", pomo_tog_lbl)?;
    api.set("card_show_title", card_show_title)?;
    api.set("card_show_glyph", card_show_glyph)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the gauges module reads — the memory + disk bead
/// rings' live figures (the geometry ladders stay in the module, mirroring the
/// `gauges.ron` `When` windows and `fit`/`dot_ladder`). `mem_lit`/`disk_lit`
/// are pre-resolved in f32 here so the Lua double arithmetic can't drift off
/// the scene's bead count. Pure: everything by value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_gauges_lua_api(
    lua: &Lua,
    mem_pct: i32,
    mem_gb: &str,
    mem_lit: i32,
    disk_pct: i32,
    disk_gb: &str,
    disk_lit: i32,
    mem_fill: u32,
    disk_fill: u32,
    dim: u32,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("mem_pct", mem_pct)?;
    api.set("mem_pct_txt", format!("{mem_pct}%"))?;
    api.set("mem_gb", mem_gb)?;
    api.set("mem_lit", mem_lit)?;
    api.set("disk_pct", disk_pct)?;
    api.set("disk_pct_txt", format!("{disk_pct}%"))?;
    api.set("disk_gb", disk_gb)?;
    api.set("disk_lit", disk_lit)?;
    api.set("mem_fill", mem_fill)?;
    api.set("disk_fill", disk_fill)?;
    api.set("dim", dim)?;
    Ok(Value::Table(api))
}

/// Build the `ctx.api` table the battery module (batteryh / batteryv) reads — the horizontal
/// Battery card's whole backend state, published to Lua each frame. Pure:
/// everything is taken by value (or snapshot) so the shell borrow is released
/// before the host runs. Mirrors the `scene_values` battery bindings.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_battery_lua_api(
    lua: &Lua,
    pct: i32,
    pct_txt: &str,
    ink: u32,
    pane: usize,
    present: bool,
    hover_dots: bool,
    rows: &[(String, String)],
    psave_bg: u32,
    psave_ink: u32,
    psave_lbl: &str,
    card_show_title: bool,
    card_show_glyph: bool,
) -> mlua::Result<Value> {
    let api = lua.create_table()?;
    api.set("pct", pct)?;
    api.set("pct_txt", pct_txt)?;
    api.set("ink", ink)?;
    api.set("pane", pane as i64)?;
    api.set("present", present)?;
    api.set("hover_dots", hover_dots)?;
    let list = lua.create_table()?;
    for (i, (label, value)) in rows.iter().enumerate() {
        let row = lua.create_table()?;
        row.set("label", label.as_str())?;
        row.set("value", value.as_str())?;
        list.set(i + 1, row)?;
    }
    api.set("rows", list)?;
    api.set("psave_bg", psave_bg)?;
    api.set("psave_ink", psave_ink)?;
    api.set("psave_lbl", psave_lbl)?;
    api.set("card_show_title", card_show_title)?;
    api.set("card_show_glyph", card_show_glyph)?;
    Ok(Value::Table(api))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Cmd, GridScale};

    fn pal() -> Pal {
        Pal {
            fg: 0xe8e6e3ff,
            bg: 0x141414ff,
            acc: 0x4fc2ffff,
            sfg: 0x101010ff,
        }
    }

    /// A default `SceneRender` for tests whose module never calls
    /// `ctx.ui.scene` (the binding is unused, so the box/values are irrelevant).
    fn no_scene() -> SceneRender {
        SceneRender {
            vals: Rc::new(crate::scene::SceneValues::new()),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            hover: 0,
            grid: 1.0,
            show_title: false,
            show_glyph: false,
        }
    }

    fn fmt(v: &[Cmd]) -> Vec<String> {
        v.iter()
            .map(|c| match c {
                Cmd::Text {
                    x,
                    y,
                    right,
                    center,
                    text,
                    size,
                    color,
                    ..
                } => {
                    format!("T {x:.2} {y:.2} {right} {center} {size:.2} {color} {text}")
                }
                Cmd::Rect {
                    x,
                    y,
                    w,
                    h,
                    r,
                    color,
                } => format!("R {x:.2} {y:.2} {w:.2} {h:.2} {r:.2} {color}"),
                Cmd::Outline {
                    x,
                    y,
                    w,
                    h,
                    r,
                    color,
                    ..
                } => format!("O {x:.2} {y:.2} {w:.2} {h:.2} {r:.2} {color}"),
                _ => String::new(),
            })
            .collect()
    }

    /// Write a module to a throwaway file under the target dir (never the
    /// committed `ui/cards/` tree), returning (dir, path).
    fn write_module(id: &str, code: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("zen_lua_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{id}.lua"));
        std::fs::write(&p, code).unwrap();
        p
    }

    #[test]
    fn lua_host_loads_and_paints_a_surface() {
        let p = write_module(
            "demo",
            r#"
            function draw(ctx)
                local s, fs = ctx.s, ctx.fs
                ctx.ui.surface(ctx.x + s(8), ctx.y + s(10), ctx.w - s(16), s(40), s(6), ctx.pal.hover)
                ctx.ui.text(ctx.x + s(14), ctx.y + s(20), "hello", fs(9), ctx.pal.fg, false)
                ctx.ui.textc(ctx.x + ctx.w / 2, ctx.y + ctx.h / 2, "center", fs(9), ctx.pal.fg2, false)
                ctx.ui.hit(ctx.x + s(8), ctx.y + s(10), s(20), s(20), 700)
            end
            "#,
        );
        let mut host = LuaHost::new();
        let mut v: Vec<Cmd> = Vec::new();
        let hits = host
            .draw_card("demo", &p, &mut v, 0.0, 0.0, 200.0, 120.0, 0, pal(), 1.0, &|_| {
                Ok(Value::Nil)
            }, &no_scene())
            .expect("module drew");
        assert_eq!(v.len(), 3, "surface + two texts");
        assert_eq!(hits, vec![(8.0, 10.0, 20.0, 20.0, 700)], "hit region routed");
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "hello")),
            "module text landed in the command list"
        );
    }

    #[test]
    fn lua_host_hot_reloads_on_mtime_change() {
        let p = write_module("reload", r#"function draw(ctx) ctx.ui.text(10, 10, "one", 9, 0, false) end"#);
        let mut host = LuaHost::new();
        let mut v: Vec<Cmd> = Vec::new();
        host.draw_card("reload", &p, &mut v, 0.0, 0.0, 100.0, 100.0, 0, pal(), 1.0, &|_| Ok(Value::Nil), &no_scene());
        assert!(v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "one")));
        std::fs::write(&p, r#"function draw(ctx) ctx.ui.text(10, 10, "two", 9, 0, false) end"#).unwrap();
        let mut v2: Vec<Cmd> = Vec::new();
        // ensure the mtime actually differs (elapsed microsecond writes)
        let sleep = std::time::Duration::from_millis(5);
        std::thread::sleep(sleep);
        assert!(
            host.draw_card("reload", &p, &mut v2, 0.0, 0.0, 100.0, 100.0, 0, pal(), 1.0, &|_| Ok(Value::Nil), &no_scene())
                .is_some(),
            "reload happened"
        );
        assert!(
            v2.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "two")),
            "reloaded module painted the new text"
        );
    }

    #[test]
    fn lua_host_falls_back_when_module_is_missing_or_broken() {
        let mut host = LuaHost::new();
        let path = write_module("nope", "this is not lua at all {{{{");
        std::fs::write(&path, "function draw(ctx)\n  ctx.ui.surface(0,0,1,1,1, ctx.undeclared) -- errors\nend").unwrap();
        let mut v: Vec<Cmd> = Vec::new();
        assert!(
            host.draw_card("missing", &std::env::temp_dir().join("zen_lua_test").join("absent.lua"), &mut v, 0.0, 0.0, 10.0, 10.0, 0, pal(), 1.0, &|_| Ok(Value::Nil), &no_scene())
                .is_none(),
            "missing file -> no draw"
        );
        let mut v2: Vec<Cmd> = Vec::new();
        assert!(
            host.draw_card("broken", &path, &mut v2, 0.0, 0.0, 10.0, 10.0, 0, pal(), 1.0, &|_| Ok(Value::Nil), &no_scene())
                .is_none(),
            "module whose draw errors -> no draw (fallback paints)"
        );
    }

    #[test]
    fn module_reads_the_stdlib_but_writes_stay_module_local() {
        // The module env is a sandbox for WRITES only: `tostring` / `math` /
        // `string` / `table` must resolve (they are plain `_G` reads), while a
        // top-level assignment lands in the module's own table — two modules
        // can never see each other's globals.
        let p = write_module(
            "stdlib",
            r#"
            leaked = "at load"
            function draw(ctx)
                local joined = table.concat({ "a", "b" }, "-")
                local n = math.floor(3.7) + #string.upper("x")
                leaked = "in draw"
                ctx.ui.text(0, 0, joined .. "|" .. tostring(n) .. "|" .. leaked, 9, 0, false)
            end
            "#,
        );
        let mut host = LuaHost::new();
        let mut v: Vec<Cmd> = Vec::new();
        host.draw_card("stdlib", &p, &mut v, 0.0, 0.0, 100.0, 100.0, 0, pal(), 1.0, &|_| {
            Ok(Value::Nil)
        }, &no_scene())
        .expect("module drew — the stdlib resolved");
        assert!(
            v.iter().any(|c| matches!(c, Cmd::Text { text, .. } if text == "a-b|4|in draw")),
            "string/math/table/tostring all reachable from the module env"
        );
        let e = host.envs.get("stdlib").expect("env kept alive");
        let (ke, _) = e.env.as_ref().expect("module env stored");
        let env: Table = e.lua.registry_value(ke).expect("env table");
        assert_eq!(
            env.get::<String>("leaked").expect("env read"),
            "in draw",
            "assignments during draw land in the module env"
        );
        let g: Option<String> = e.lua.globals().get::<Option<String>>("leaked").ok().flatten();
        assert_eq!(g, None, "the module's globals never reach _G");
    }

    /// Draw a card's live `.ron` scene and its `.lua` module for the same state
    /// and assert identical commands + hit regions. TEMPORARY scaffold used
    /// only while migrating a card; each per-card test is deleted with its
    /// `.ron`.
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    fn lua_vs_scene(
        id: &str,
        w: f32,
        h: f32,
        hover: u32,
        show_title: bool,
        show_glyph: bool,
        pal: Pal,
        vals: &crate::scene::SceneValues,
        api: &crate::lua::BuildApi,
    ) {
        let sp = crate::vars::card_scene_path(id);
        let src = std::fs::read_to_string(&sp).expect("scene file readable");
        let scene: crate::scene::CardScene =
            crate::scene::parse_scene_ron(&src).expect("scene parses");
        let mut sv = Vec::new();
        let (hits, _inks) = scene.draw(
            &mut sv,
            0.0,
            0.0,
            w,
            h,
            GridScale(1.0),
            &pal,
            vals,
            hover,
            show_title,
            show_glyph,
        );
        let lp = crate::vars::card_lua_path(id);
        let mut host = LuaHost::new();
        let mut lv = Vec::new();
        let lhits = host
            .draw_card(
                id,
                &lp,
                &mut lv,
                0.0,
                0.0,
                w,
                h,
                hover,
                pal,
                1.0,
                api,
                &SceneRender {
                    vals: Rc::new(vals.clone()),
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                    hover,
                    grid: 1.0,
                    show_title,
                    show_glyph,
                },
            )
            .expect("lua module drew");
        let sh: Vec<(f32, f32, f32, f32, u32)> =
            hits.into_iter().map(|x| (x.x, x.y, x.w, x.h, x.key)).collect();
        assert_eq!(fmt(&sv), fmt(&lv), "{id}: lua vs scene command mismatch");
        assert_eq!(sh, lhits, "{id}: lua vs scene hit mismatch");
    }


    }
