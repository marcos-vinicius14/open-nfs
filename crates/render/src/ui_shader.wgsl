// Open-NFS 2D UI Overlay WGSL Shader for Virtual Steering Wheel and Pedals

struct ScreenUniforms {
    screen_size: vec2<f32>,
    padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> screen: ScreenUniforms;

struct UiVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct UiVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_ui(in: UiVertexInput) -> UiVertexOutput {
    var out: UiVertexOutput;
    // Map screen pixel coords (x in [0, w], y in [0, h]) to Vulkan NDC ([-1, 1], [-1, 1])
    let ndc_x = (in.position.x / screen.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.position.y / screen.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.5, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_ui(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
