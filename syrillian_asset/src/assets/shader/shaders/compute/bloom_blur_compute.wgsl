struct BloomParams {
    threshold: f32,
    soft_knee: f32,
    intensity: f32,
    radius: f32,
    clamp_max: f32,
    _pad0: f32,
    direction: vec2f,
    texel_size: vec2f,
}

@group(0) @binding(0) var bloomInput: texture_2d<f32>;
@group(0) @binding(1) var bloomAuxInput: texture_2d<f32>;
@group(0) @binding(2) var bloomSampler: sampler;
@group(0) @binding(3) var<uniform> bloomParams: BloomParams;
@group(0) @binding(4) var bloomOutput: texture_storage_2d<rgba8unorm, write>;

fn sample_rgb(uv: vec2f) -> vec3f {
    return textureSampleLevel(bloomInput, bloomSampler, uv, 0.0).rgb;
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let out_size = textureDimensions(bloomOutput);
    if (gid.x >= out_size.x || gid.y >= out_size.y) {
        return;
    }

    let uv = (vec2f(gid.xy) + vec2f(0.5)) / vec2f(out_size);
    let delta = bloomParams.direction * bloomParams.texel_size * max(bloomParams.radius, 0.0001);

    var color = sample_rgb(uv) * 0.227027;
    color += sample_rgb(uv + delta * 1.0) * 0.1945946;
    color += sample_rgb(uv - delta * 1.0) * 0.1945946;
    color += sample_rgb(uv + delta * 2.0) * 0.1216216;
    color += sample_rgb(uv - delta * 2.0) * 0.1216216;
    color += sample_rgb(uv + delta * 3.0) * 0.054054;
    color += sample_rgb(uv - delta * 3.0) * 0.054054;
    color += sample_rgb(uv + delta * 4.0) * 0.016216;
    color += sample_rgb(uv - delta * 4.0) * 0.016216;

    if (bloomParams.clamp_max > 0.0) {
        color = min(color, vec3f(bloomParams.clamp_max));
    }

    textureStore(bloomOutput, vec2i(gid.xy), vec4f(color, 1.0));
}
