struct CustomMaterial {
    base_positions: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@group(1) @binding(1)
var layer_1: texture_2d<f32>;
@group(1) @binding(2)
var layer_1_sampler: sampler;

@group(1) @binding(3)
var alpha_1: texture_2d<f32>;
@group(1) @binding(4)
var alpha_1_sampler: sampler;

@group(1) @binding(5)
var layer_2: texture_2d<f32>;
@group(1) @binding(6)
var layer_2_sampler: sampler;

@group(1) @binding(7)
var alpha_2: texture_2d<f32>;
@group(1) @binding(8)
var alpha_2_sampler: sampler;

@group(1) @binding(9)
var layer_3: texture_2d<f32>;
@group(1) @binding(10)
var layer_3_sampler: sampler;

@group(1) @binding(11)
var alpha_3: texture_2d<f32>;
@group(1) @binding(12)
var alpha_3_sampler: sampler;

@group(1) @binding(13)
var layer_4: texture_2d<f32>;
@group(1) @binding(14)
var layer_4_sampler: sampler;

@group(1) @binding(15)
var alpha_4: texture_2d<f32>;
@group(1) @binding(16)
var alpha_4_sampler: sampler;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let distance = uv - material.base_positions.xy;

    let x_n = distance.x / material.base_positions.z;
    let y_n = distance.y / material.base_positions.w;

    let uv_alpha = vec2<f32>(x_n, y_n);

    var layer_1_color: vec4<f32> = textureSample(layer_1, layer_1_sampler, uv);
    var alpha_1_value: f32 = textureSample(alpha_1, alpha_1_sampler, uv_alpha).r;

    var layer_2_color: vec4<f32> = textureSample(layer_2, layer_2_sampler, uv);
    var alpha_2_value: f32 = textureSample(alpha_2, alpha_2_sampler, uv_alpha).r;

    var layer_3_color: vec4<f32> = textureSample(layer_3, layer_3_sampler, uv);
    var alpha_3_value: f32 = textureSample(alpha_3, alpha_3_sampler, uv_alpha).r;

    var layer_4_color: vec4<f32> = textureSample(layer_4, layer_4_sampler, uv);
    var alpha_4_value: f32 = textureSample(alpha_4, alpha_4_sampler, uv_alpha).r;

    var final_color = layer_1_color * alpha_1_value; // + layer_2_color * alpha_2_value + layer_3_color * alpha_3_value + layer_4_color * alpha_4_value;
    // var final_color: vec4<f32> = layer_1_color * alpha_1_value;

    // if (layer_2_color.x != 1.0 || layer_2_color.y != 1.0 || layer_2_color.z != 1.0) {
    //     final_color = final_color + (layer_2_color * alpha_2_value);
    // }

    // if (layer_3_color.x != 1.0 || layer_3_color.y != 1.0 || layer_3_color.z != 1.0) {
    //     final_color = final_color + (layer_3_color * alpha_3_value);
    // }

    // if (layer_4_color.x != 1.0 || layer_4_color.y != 1.0 || layer_4_color.z != 1.0) {
    //     final_color = final_color + (layer_4_color * alpha_4_value);
    // }

    return (final_color);
}
