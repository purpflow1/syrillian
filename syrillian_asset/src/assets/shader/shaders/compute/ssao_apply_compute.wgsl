@group(0) @binding(0) var colorInput: texture_2d<f32>;
@group(0) @binding(1) var aoInput: texture_2d<f32>;
@group(0) @binding(2) var colorOutput: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3u) {
    let size = textureDimensions(colorOutput);
    if (gid.x >= size.x || gid.y >= size.y) {
        return;
    }

    let p = vec2i(gid.xy);
    let base = textureLoad(colorInput, p, 0);
    let ao = clamp(textureLoad(aoInput, p, 0).x, 0.0, 1.0);
    textureStore(colorOutput, p, vec4f(base.rgb * ao, base.a));
}
