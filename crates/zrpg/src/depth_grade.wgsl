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
    // x: how far it has dissolved (0..1), y: the pixel grid it breaks up on,
    // z: the force of a blow landing (0..1, already fading), w: seconds since.
    dream: vec4<f32>,
}

// Where on a sprite blows land, in its quad's UVs: about the chest.
const IMPACT_AT: vec2<f32> = vec2<f32>(0.5, 0.45);

// A blow landing: a shockwave ring spreads out from the chest, bending the
// picture outward as it passes, while the whole body shudders side to
// side. Both scale with the force. Returns the bent UV and how bright the
// ring is here.
fn impact(uv: vec2<f32>, force: f32, age: f32) -> vec3<f32> {
    let d = uv - IMPACT_AT;
    let r = length(d);
    let radius = age * 1.8;
    let ring = exp(-pow((r - radius) / 0.07, 2.0));
    let push = d / max(r, 0.001) * ring * force * 0.04;
    let shudder = sin(uv.y * 48.0 + age * 95.0) * force * 0.012;
    // Stay inside this frame of the sheet.
    let bent = clamp(uv - push + vec2<f32>(shudder, 0.0), vec2<f32>(0.004), vec2<f32>(0.996));
    // Only the spreading ring catches the light, not the point it starts.
    return vec3<f32>(bent, ring * force * smoothstep(0.03, 0.15, radius));
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> grade: DepthGrade;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // A blow bends the picture before it is sampled.
    var bent_in = in;
    var impact_glow = 0.0;
#ifdef VERTEX_UVS_A
    if (grade.dream.z > 0.0) {
        let hit = impact(in.uv, grade.dream.z, grade.dream.w);
        bent_in.uv = hit.xy;
        impact_glow = hit.z;
    }
#endif
    var pbr_input = pbr_input_from_standard_material(bent_in, is_front);
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
    // The shockwave catches the light a little as it passes.
    out.color = vec4<f32>(out.color.rgb + vec3<f32>(1.0, 0.9, 0.8) * 0.35 * impact_glow, out.color.a);

    // Fog, alpha handling and in-shader tonemapping, as in the standard shader.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
