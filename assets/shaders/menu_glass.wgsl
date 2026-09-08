#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct MenuGlassSettings {
    geometry: vec4<f32>,
}

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: MenuGlassSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = max(settings.geometry.xy, vec2<f32>(1.0));
    let pixel = in.uv * size;
    let under_header = pixel.y < settings.geometry.z;
    let under_drawer = pixel.x >= size.x - settings.geometry.w;
    if !under_header && !under_drawer {
        return textureSampleLevel(scene, scene_sampler, in.uv, 0.0);
    }

    // Separable Gaussian frost (18 logical-pixel sigma, 40-pixel radius).
    // Dense samples avoid stepping at edges; UI text is drawn after both passes.
    var color = vec4<f32>(0.0);
    var total = 0.0;
    for (var i = -20; i <= 20; i += 1) {
        let distance = f32(i) * 2.0;
        let weight = exp(-distance * distance / (2.0 * 18.0 * 18.0));
#ifdef BLUR_HORIZONTAL
        let uv = in.uv + vec2<f32>(distance / size.x, 0.0);
#else
        var uv = in.uv + vec2<f32>(0.0, distance / size.y);
        if under_header && !under_drawer {
            // The horizontal pass only frosts chrome. Extend the header's edge
            // instead of pulling sharp, unprocessed scene pixels into this pass.
            uv.y = min(uv.y, (settings.geometry.z - 0.5) / size.y);
        }
#endif
        color += textureSampleLevel(scene, scene_sampler, uv, 0.0) * weight;
        total += weight;
    }
    return color / total;
}
