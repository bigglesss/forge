struct CustomMaterial {
    base_positions: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@group(1) @binding(1)
var layer_1: texture_2d<f32>;
@group(1) @binding(2)
var layer_1_sampler: sampler;


@group(1) @binding(3)
var layer_2: texture_2d<f32>;
@group(1) @binding(4)
var layer_2_sampler: sampler;

@group(1) @binding(5)
var alpha_2: texture_2d<f32>;
@group(1) @binding(6)
var alpha_2_sampler: sampler;

@group(1) @binding(7)
var layer_3: texture_2d<f32>;
@group(1) @binding(8)
var layer_3_sampler: sampler;

@group(1) @binding(9)
var alpha_3: texture_2d<f32>;
@group(1) @binding(10)
var alpha_3_sampler: sampler;

@group(1) @binding(11)
var layer_4: texture_2d<f32>;
@group(1) @binding(12)
var layer_4_sampler: sampler;

@group(1) @binding(13)
var alpha_4: texture_2d<f32>;
@group(1) @binding(14)
var alpha_4_sampler: sampler;

fn saturation(color: vec4<f32>, adjustment: f32) -> vec4<f32>
{
    // Algorithm from Chapter 16 of OpenGL Shading Language
    let W: vec4<f32> = vec4(0.2125, 0.7154, 0.0721, 1.0);
    let intensity: vec4<f32> = vec4(dot(color, W));
    return mix(intensity, color, adjustment);
}

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let distance_from_origin = uv - material.base_positions.xy;

    // For some reason x + y are flipped here, perhaps I made a mistake somewhere.
    let uv_alpha = vec2<f32>(abs(distance_from_origin.y) / 33.333496, abs(distance_from_origin.x) / 33.333496);

    var layer_1_color: vec4<f32> = textureSample(layer_1, layer_1_sampler, uv);

    var layer_2_color: vec4<f32> = textureSample(layer_2, layer_2_sampler, uv);
    var alpha_2_value: f32 = textureSample(alpha_2, alpha_2_sampler, uv_alpha).r;

    var layer_3_color: vec4<f32> = textureSample(layer_3, layer_3_sampler, uv);
    var alpha_3_value: f32 = textureSample(alpha_3, alpha_3_sampler, uv_alpha).r;

    var layer_4_color: vec4<f32> = textureSample(layer_4, layer_4_sampler, uv);
    var alpha_4_value: f32 = textureSample(alpha_4, alpha_4_sampler, uv_alpha).r;

    // finalColor = tex0 * (1.0 - (alpha1 + alpha2 + alpha3)) + tex1 * alpha1 + tex2 * alpha2 + tex3 * alpha3
    var final_color: vec4<f32> = layer_1_color * (1.0 - (alpha_2_value + alpha_3_value + alpha_4_value)) + (layer_2_color * alpha_2_value) + (layer_3_color * alpha_3_value) + (layer_4_color * alpha_4_value);

    // return layer_1_color * (1.0 - (alpha_2_value + alpha_3_value + alpha_4_value)) + (layer_2_color * alpha_2_value);

    return saturation(final_color * (world_normal.y / 2.0), 1.25);
}
