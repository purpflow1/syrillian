#use light

struct SkyVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> SkyVertexOut {
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );

    let uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );

    var out: SkyVertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: SkyVertexOut) -> @location(0) vec4<f32> {
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let world_far_h = camera.inv_view_proj_mat * vec4<f32>(ndc, 1.0, 1.0);
    let world_far = world_far_h.xyz / max(world_far_h.w, 1e-6);
    let dir = normalize(world_far - camera.position);

    let sun_dir = sky_sun_direction();
    let atm = atmosphere_eval(dir, sun_dir);

    let env = textureSample(skybox_map, skybox_sampler, dir).rgb;

    let hemi = smoothstep(-0.02, 0.02, dir.y);

    let atm_applied = env * atm.transmittance_view + atm.sky_radiance;
    let sky_color = mix(env, atm_applied, hemi);

    let sun = sky_sun_disk_add(dir, sun_dir, sky_color, atm);

    return vec4<f32>(sky_color + sun, 1.0);
}
