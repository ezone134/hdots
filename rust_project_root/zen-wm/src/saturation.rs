//! Global saturation via a custom GLES texture shader.
//!
//! The default smithay texture shader is overridden per-frame with a variant that
//! desaturates/saturates all rendered textures in the same single pass — no extra
//! render targets, no second blit. The shader is the upstream `texture.frag`
//! (kept byte-for-byte compatible with the `//_DEFINES_` + `EXTERNAL`/`NO_ALPHA`/
//! `DEBUG_FLAGS` variant machinery) plus a `saturation` uniform.

use smithay::backend::renderer::gles::{GlesFrame, GlesRenderer, GlesTexProgram, Uniform, UniformName, UniformType};

/// Default smithay texture fragment shader with a saturation mix added.
/// `//_DEFINES_` is replaced by `compile_custom_texture_shader`.
pub const SATURATION_FRAGMENT: &str = r#"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision mediump float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
uniform float saturation;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

void main() {
    vec4 color = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3(luma), color.rgb, saturation);

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
"#;

/// Compile the saturation shader once per renderer (texture programs are per-context).
pub fn compile_saturation_program(renderer: &mut GlesRenderer) -> Option<GlesTexProgram> {
    match renderer.compile_custom_texture_shader(
        SATURATION_FRAGMENT,
        &[UniformName::new("saturation", UniformType::_1f)],
    ) {
        Ok(program) => Some(program),
        Err(e) => {
            tracing::error!(error = %e, "failed to compile saturation shader, disabling saturation");
            None
        }
    }
}

/// Apply the saturation override to a frame. Must be called right after `clear` and
/// before any element drawing; affects every `render_texture_*` call in this frame.
pub fn apply_saturation(frame: &mut GlesFrame<'_, '_>, program: &GlesTexProgram, saturation: f32) {
    frame.override_default_tex_program(
        program.clone(),
        vec![Uniform::new("saturation", saturation).into_owned()],
    );
}
