// Depth grading for lit surfaces: Bevy's standard PBR shading, then two
// distance effects before the built-in fog and tonemapping run:
//
// - desaturation: colours drain toward a cool grey with distance;
// - darkness: brightness falls away with distance (on top of light falloff).
//
// Mirrors bevy_pbr's own `pbr.wgsl` fragment entry point; see
// `atmosphere.rs` for the parameters.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::view,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

struct DepthGrade {
    // x: distance where grading starts, y: distance where it is complete,
    // z: desaturation at full distance (0..1), w: brightness at full distance.
    params: vec4<f32>,
    // rgb: the tint far-away colours drain toward (multiplied into the grey).
    tint: vec4<f32>,
    // x: how far it has dissolved (0..1), y: the pixel grid it breaks up on.
    dream: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> grade: DepthGrade;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // Dissolving like a dream: pixels drop out in a fixed random order; the
    // ones about to go glow pale first.
    var dream_glow = 0.0;
#ifdef VERTEX_UVS_A
    if (grade.dream.x > 0.0) {
        let cell = floor(in.uv * grade.dream.y);
        let h = fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        if (h < grade.dream.x) {
            discard;
        }
        dream_glow = 1.0 - smoothstep(grade.dream.x, grade.dream.x + 0.18, h);
    }
#endif

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    let d = distance(in.world_position.xyz, view.world_position.xyz);
    let t = smoothstep(grade.params.x, grade.params.y, d);
    let luma = dot(out.color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let drained = mix(out.color.rgb, vec3<f32>(luma) * grade.tint.rgb, t * grade.params.z);
    out.color = vec4<f32>(drained * mix(1.0, grade.params.w, t), out.color.a);
    out.color = vec4<f32>(out.color.rgb + vec3<f32>(0.7, 0.85, 1.0) * 1.6 * dream_glow, out.color.a);

    // Fog, alpha handling and in-shader tonemapping, as in the standard shader.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
