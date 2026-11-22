// A portal's aperture: the portal camera's image, texel for pixel, read where this fragment lies in
// the window. The image's corner is at `origin`.

import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> origin: vec4<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(1)
var image: texture_2d<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let last = vec2<i32>(textureDimensions(image)) - 1;
    let texel = clamp(vec2<i32>(floor(in.position.xy - origin.xy)), vec2<i32>(0), last);
    return vec4(textureLoad(image, texel, 0).rgb, 1.0);
}
