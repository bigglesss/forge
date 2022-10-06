@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    return vec4(0.2, 0.2, 0.6, 0.75);
}
