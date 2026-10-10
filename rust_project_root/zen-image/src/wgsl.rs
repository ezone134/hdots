pub const VERTEX_WGSL: &str = r#"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Globals {
    scale: vec2<f32>,
    offset: vec2<f32>,
    texel: vec2<f32>,
    misc: vec2<f32>, // x = quarter turns (0..3), y = unused
}

@group(0) @binding(2) var<uniform> globals: Globals;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let ndc = in.position * 2.0 - 1.0;
    out.clip = vec4<f32>(ndc * globals.scale + globals.offset, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}
"#;

pub const FRAGMENT_WGSL: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct Globals {
    scale: vec2<f32>,
    offset: vec2<f32>,
    texel: vec2<f32>,   // 1.0 / (width, height)
    misc: vec2<f32>,    // x = quarter turns, y = unused
}

@group(0) @binding(2) var<uniform> globals: Globals;

fn checker(p: vec2<f32>) -> f32 {
    let q = floor(p);
    return select(0.36, 0.44, (q.x + q.y) % 2.0 < 0.5);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // rotate texture coordinates by quarter turns: uv in [0,1]^2 stays in range
    var t = uv;
    let turns = u32(globals.misc.x + 0.5) % 4u;
    if (turns == 1u) {
        t = vec2<f32>(1.0 - uv.y, uv.x);
    } else if (turns == 2u) {
        t = vec2<f32>(1.0 - uv.x, 1.0 - uv.y);
    } else if (turns == 3u) {
        t = vec2<f32>(uv.y, 1.0 - uv.x);
    }

    let c = textureSampleLevel(tex, smp, t, 0.0);

    if (c.a < 1.0) {
        let grid = floor(uv / vec2<f32>(0.015625));
        let shade = checker(grid);
        let bg = vec3<f32>(shade * 0.28 + 0.05);
        return vec4<f32>(mix(bg, c.rgb, c.a), 1.0);
    }
    return vec4<f32>(c.rgb, 1.0);
}
"#;

pub fn compile(wgsl: &str) -> Vec<u32> {
    let module = naga::front::wgsl::parse_str(wgsl).expect("wgsl parse");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("wgsl validate");
    naga::back::spv::write_vec(&module, &info, &Default::default(), None).expect("spv emit")
}
