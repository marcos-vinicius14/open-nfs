// Open-NFS Vehicle WGSL Shader: Diffuse texturing + Vertex coloring + Directional lighting

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = camera.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_normal = normalize((camera.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.color = in.color;
    out.uv = in.uv;
    out.world_pos = world_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    if (tex_color.a < 0.2) {
        discard;
    }

    // Normalize surface normal and lighting vectors
    let n_len = length(in.world_normal);
    var N = vec3<f32>(0.0, 1.0, 0.0);
    if (n_len > 0.001) {
        N = in.world_normal / n_len;
    }

    let L = normalize(camera.light_dir.xyz);
    let V = normalize(camera.camera_pos.xyz - in.world_pos);
    let H = normalize(L + V);

    // Hemisphere ambient (sky top + ground bounce) + directional key light + rim/fill light
    let hemi = 0.5 + 0.5 * N.y;
    let ambient = mix(0.28, 0.52, hemi);
    let ndotl = max(dot(N, L), 0.0);
    let fill = max(dot(N, -L), 0.0) * 0.15;
    let diffuse_light = ambient + ndotl * 0.68 + fill;

    // Subtle automotive Blinn-Phong specular highlight
    let ndoth = max(dot(N, H), 0.0);
    let spec = pow(ndoth, 28.0) * 0.28 * step(0.0, dot(N, L));

    // In NFSU2 GEOMETRY.BIN, Vertex36.color stores either 0xFF000000 (unlit/zero RGB)
    // or channel-packed shader masks (high R byte, low G/B). Do not multiply raw RGB
    // directly or it will turn parts pitch black and red. Use a gentle luma AO term instead.
    let v_luma = dot(in.color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let ao = mix(0.90, 1.0, clamp(v_luma, 0.0, 1.0));

    let lit_rgb = clamp(tex_color.rgb * ao * diffuse_light + vec3<f32>(spec), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(lit_rgb, 1.0);
}
