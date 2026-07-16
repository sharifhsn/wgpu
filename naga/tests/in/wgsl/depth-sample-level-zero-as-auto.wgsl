@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var comparison_sampler: sampler_comparison;

@fragment
fn main() -> @location(0) f32 {
    return textureSampleCompareLevel(
        depth_texture,
        comparison_sampler,
        vec2(0.5),
        0.5,
    );
}
