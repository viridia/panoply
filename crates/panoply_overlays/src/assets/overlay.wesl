// A flat color: overlays are drawn unlit, so they read the same in any light.

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> color: vec4<f32>;

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return color;
}
