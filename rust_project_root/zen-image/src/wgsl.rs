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

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(tex, smp, uv);
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