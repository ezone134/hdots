-- APPS — declarative launcher slate. The `Grid` renders the 8-slot square-tile
-- matrix from `app_cells` (resting tile fill = the `app_tile` /
-- `app_tile_empty` per-frame blends, raster icons or the apps-glyph fallback,
-- bottom labels, keys 34400+i, hover-✕ remove at 34416+i); header = apps glyph
-- + title + live `n/8` count.
local SCENE = [==[
(
    items: [
        Header(title: "Apps", meta: "{app_count}", glyph: "\u{f489}"),
        Grid(
            name: "app_cells",
            x: 12.0, y: 36.0, w: -24.0, h: -46.0,
            square: true,
            gap: 8.0,
            r: 10.0,
            font_size: 8.0,
            image_size: 26.0, image_top: 6.0,
            del_base: 34416,
            key_base: 34400,
        ),
    ],
)
]==]

function draw(ctx)
    ctx.ui.scene(SCENE)
end
