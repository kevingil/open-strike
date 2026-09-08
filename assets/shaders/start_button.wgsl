#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var<uniform> animation: vec4<f32>;

fn hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

fn noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let blend = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let lower = mix(
        mix(hash(cell), hash(cell + vec3<f32>(1.0, 0.0, 0.0)), blend.x),
        mix(hash(cell + vec3<f32>(0.0, 1.0, 0.0)), hash(cell + vec3<f32>(1.0, 1.0, 0.0)), blend.x),
        blend.y,
    );
    let upper = mix(
        mix(hash(cell + vec3<f32>(0.0, 0.0, 1.0)), hash(cell + vec3<f32>(1.0, 0.0, 1.0)), blend.x),
        mix(hash(cell + vec3<f32>(0.0, 1.0, 1.0)), hash(cell + vec3<f32>(1.0, 1.0, 1.0)), blend.x),
        blend.y,
    );
    return mix(lower, upper, blend.z);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let time = animation.x;
    let hover = animation.y;
    let pressed = animation.z;
    let aspect = in.size.x / max(in.size.y, 1.0);
    // Evolving noise changes the contour shapes as they travel. Independent time
    // scales and curved drift keep the motion varied, continuous and free of jumps.
    let p = vec2<f32>(in.uv.x * aspect, in.uv.y) * 2.0
        + vec2<f32>(time * 0.24 + 0.35 * sin(time * 0.71), time * 0.1 + 0.25 * cos(time * 0.53));
    let terrain = noise(vec3<f32>(p * 0.75, time * 0.32))
        + 0.42 * noise(vec3<f32>(p * 1.45 + vec2<f32>(14.0, 9.0), -time * 0.43));
    let elevation = terrain * 18.0;
    let distance_to_line = abs(fract(elevation + 0.5) - 0.5);
    let line_width = max(fwidth(elevation), 0.001);
    let contour = 1.0 - smoothstep(line_width * 0.35, line_width * 1.25, distance_to_line);

    let glow = 0.5 + 0.5 * sin(p.x * 0.65 - p.y * 0.45 + time * 0.22);
    var color = mix(vec3<f32>(0.002, 0.035, 0.001), vec3<f32>(0.008, 0.095, 0.002), glow);
    color += contour * vec3<f32>(0.006, 0.075, 0.0015);
    color *= 1.0 + hover * 0.45 - pressed * 0.3;

    // A crisp green frame with a soft inner glow, without moving the hit target.
    let edge = min(in.uv, vec2<f32>(1.0) - in.uv) * in.size;
    let distance_to_edge = min(edge.x, edge.y);
    color += exp(-distance_to_edge * 0.24)
        * vec3<f32>(0.005 + hover * 0.015, 0.045 + hover * 0.10, 0.001);
    let border = 1.0 - smoothstep(0.7, 1.5, distance_to_edge);
    color = mix(color, vec3<f32>(0.025 + hover * 0.07, 0.38 + hover * 0.28, 0.003), border);
    return vec4<f32>(color, 1.0);
}
