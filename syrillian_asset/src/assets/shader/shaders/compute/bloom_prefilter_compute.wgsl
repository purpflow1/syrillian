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

fn extract_bright(color: vec3f) -> vec3f {
    let brightness = max(max(color.r, color.g), color.b);
    let knee = max(bloomParams.soft_knee, 0.0001);
    let threshold = bloomParams.threshold;
    let diff = brightness - threshold;
    let soft = clamp(diff + knee, 0.0, 2.0 * knee);
    let soft_term = (soft * soft) / (4.0 * knee + 0.0001);
    let contrib = max(diff, soft_term) / max(brightness, 0.0001);
    return color * clamp(contrib, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let out_size = textureDimensions(bloomOutput);
    if (gid.x >= out_size.x || gid.y >= out_size.y) {
        return;
    }

    let out_size_f = vec2f(out_size);
    let uv = (vec2f(gid.xy) + vec2f(0.5)) / out_size_f;
    let input_size = vec2f(textureDimensions(bloomInput, 0));
    let input_texel = 1.0 / input_size;

    let c0 = textureSampleLevel(bloomInput, bloomSampler, uv + input_texel * vec2f(-0.5, -0.5), 0.0).rgb;
    let c1 = textureSampleLevel(bloomInput, bloomSampler, uv + input_texel * vec2f(0.5, -0.5), 0.0).rgb;
    let c2 = textureSampleLevel(bloomInput, bloomSampler, uv + input_texel * vec2f(-0.5, 0.5), 0.0).rgb;
    let c3 = textureSampleLevel(bloomInput, bloomSampler, uv + input_texel * vec2f(0.5, 0.5), 0.0).rgb;
    var color = (c0 + c1 + c2 + c3) * 0.25;
    color = extract_bright(color);

    if (bloomParams.clamp_max > 0.0) {
        color = min(color, vec3f(bloomParams.clamp_max));
    }

    textureStore(bloomOutput, vec2i(gid.xy), vec4f(color, 1.0));
}
