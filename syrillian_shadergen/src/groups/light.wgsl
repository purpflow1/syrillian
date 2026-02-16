const LIGHT_TYPE_POINT: u32 = 0;
const LIGHT_TYPE_SUN: u32 = 1;
const LIGHT_TYPE_SPOT: u32 = 2;

struct Light {
    position: vec3<f32>,
    up: vec3<f32>,
    radius: f32,
    direction: vec3<f32>,
    range: f32,
    color: vec3<f32>,
    intensity: f32,
    inner_angle: f32,
    outer_angle: f32,
    cos_inner: f32,
    cos_outer: f32,
    type_id: u32,
    shadow_map_id: u32,
    shadow_mat_base: u32,
}

@group(3) @binding(0) var<uniform> light_count: u32;
@group(3) @binding(1) var<storage, read> lights: array<Light>;

@group(4) @binding(0) var shadow_maps: texture_depth_2d_array;
@group(4) @binding(1) var shadow_sampler: sampler_comparison;
@group(4) @binding(2) var<storage, read> shadow_mats: array<mat4x4<f32>>;
@group(4) @binding(3) var<uniform> shadow_texel: vec2<f32>;

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn luma(c: vec3<f32>) -> f32 { return dot(c, LUMA); }

fn sky_sun_direction() -> vec3<f32> {
    let dir_len2 = dot(sky.sun_direction, sky.sun_direction);
    if (dir_len2 > 1e-6) {
        return normalize(sky.sun_direction);
    }

    let ce = cos(sky.sun_elevation);
    return normalize(vec3<f32>(
        ce * sin(sky.sun_rotation),
        sin(sky.sun_elevation),
        -ce * cos(sky.sun_rotation)
    ));
}

fn sky_sun_color_base(sun_dir: vec3<f32>) -> vec3<f32> {
    let elev01 = saturate(sun_dir.y); // 0 at horizon, 1 at zenith
    let warm = mix(vec3<f32>(1.00, 0.60, 0.40), vec3<f32>(1.00, 0.98, 0.95), elev01);

    let haze = saturate(max(sky.air_density, 0.0) * 0.25 + max(sky.aerosols, 0.0) * 0.75);
    let neutral = vec3<f32>(luma(warm));
    return mix(warm, neutral, haze * 0.22);
}

fn rayleigh_phase(mu: f32) -> f32 {
    return (3.0 / (16.0 * PI)) * (1.0 + mu * mu);
}

fn hg_phase(mu: f32, g: f32) -> f32 {
    let gg = g * g;
    let denom = pow(max(1.0 + gg - 2.0 * g * mu, 1e-4), 1.5);
    return (1.0 - gg) / (4.0 * PI * denom);
}

fn air_mass(cos_zenith: f32) -> f32 {
    let cz = clamp(cos_zenith, 0.0, 1.0);
    let z_deg = acos(cz) * RAD_TO_DEG;
    let denom = cz + 0.50572 * pow(max(96.07995 - z_deg, 0.0001), -1.6364);
    return 1.0 / max(denom, 0.02);
}

struct AtmosphereEval {
    transmittance_view: vec3<f32>,
    transmittance_sun: vec3<f32>,
    sky_radiance: vec3<f32>,
    sun_color: vec3<f32>,
    haze: f32,
};

const HR: f32 = 8000.0;
const HM: f32 = 1200.0;
const HR_KM: f32 = HR * 0.001;
const mie_len_ratio: f32 = HM / HR;

fn atmosphere_eval(view_dir: vec3<f32>, sun_dir: vec3<f32>) -> AtmosphereEval {
    let air = max(sky.air_density, 0.0);
    let aerosol = max(sky.aerosols, 0.0);
    let haze = saturate(air * 0.35 + aerosol * 0.65);

    let alt = max(sky.altitude, 0.0);
    let rayleigh_density = exp(-alt / HR);
    let mie_density = exp(-alt / HM);

    let rayleigh_scale = air * rayleigh_density;
    let mie_scale = (aerosol + air * 0.05) * mie_density;

    let betaR = vec3<f32>(0.0058, 0.0135, 0.0331) * rayleigh_scale;

    let betaM = vec3<f32>(0.0200) * mie_scale * mie_len_ratio;

    let betaExt = betaR + betaM + vec3<f32>(1e-6);

    let m_view = air_mass(saturate(view_dir.y));
    let m_sun = air_mass(saturate(sun_dir.y));

    let L_view = m_view * HR_KM;
    let L_sun = m_sun * HR_KM;

    let T_view = exp(-betaExt * L_view);
    let T_sun = exp(-betaExt * L_sun);

    let mu = clamp(dot(view_dir, sun_dir), -1.0, 1.0);
    let pr = rayleigh_phase(mu);
    let g = mix(0.76, 0.92, haze);
    let pm = hg_phase(mu, g);

    let sun_color = sky_sun_color_base(sun_dir) * T_sun;

    let sun_strength = max(sky.sun_strength, 0.0);
    let sun_light = sun_color * sun_strength;

    let betaS = betaR * pr + betaM * pm;
    var sky_L = sun_light * 5.0 * betaS * (vec3<f32>(1.0) - T_view) / betaExt;

    let multi = sun_light * (vec3<f32>(0.015) + betaR * 0.20) * (vec3<f32>(1.0) - T_view);
    sky_L += multi;

    let horizon = 1.0 - saturate(view_dir.y);
    let fog = 1.0 - exp(-(air * 0.55 + aerosol * 1.35) * horizon * horizon * 1.15);
    let grey = vec3<f32>(luma(sky_L));
    sky_L = mix(sky_L, grey, saturate(fog * 0.60));

    let hemi = smoothstep(-0.05, 0.00, view_dir.y);
    sky_L *= hemi;

    return AtmosphereEval(T_view, T_sun, sky_L, sun_color, haze);
}

fn sky_sun_disk_add(view_dir: vec3<f32>, sun_dir: vec3<f32>, sky_color: vec3<f32>, atm: AtmosphereEval) -> vec3<f32> {
    let sun_strength = max(sky.sun_strength, 0.0);
    let sun_intensity = max(sky.sun_intensity, 0.0);

    if (sun_strength <= 0.0 || sun_intensity <= 0.0) {
        return vec3<f32>(0.0);
    }

    if (sun_dir.y <= 0.0) {
        return vec3<f32>(0.0);
    }

    let d = clamp(dot(view_dir, sun_dir), -1.0, 1.0);
    let aa = max(fwidth(d) * 2.0, 1e-6);

    let sun_disk_cos: f32 = 0.9999892;

    let sun_halo_cos: f32 = mix(0.99990, 0.99920, atm.haze);

    if (d <= sun_halo_cos) {
        return vec3<f32>(0.0);
    }

    let disk = smoothstep(sun_disk_cos - aa, sun_disk_cos + aa, d);

    let halo_t = saturate((d - sun_halo_cos) / max(sun_disk_cos - sun_halo_cos, 1e-6));
    let halo_wide  = pow(halo_t, mix(1.6, 2.6, atm.haze)) * (1.0 - disk);
    let halo_tight = pow(halo_t, mix(8.0, 14.0, atm.haze)) * (1.0 - disk);

    let center_t = saturate((d - sun_disk_cos) / max(1.0 - sun_disk_cos, 1e-6));
    let limb = mix(0.65, 1.0, pow(center_t, 0.35));

    let core = atm.sun_color;
    let halo_color = mix(core, sky_color, mix(0.55, 0.75, atm.haze));

    let direct = sun_strength * sun_intensity;

    let disk_gain   = mix(7.0, 3.5, atm.haze);
    let corona_gain = mix(0.25, 0.75, atm.haze);
    let halo_gain   = mix(0.06, 0.22, atm.haze);

    return direct * (
        core * (disk * disk_gain * limb) +
        halo_color * (halo_tight * corona_gain + halo_wide * halo_gain)
    );
}

fn sun_transmittance_rgb(sun_dir: vec3<f32>) -> vec3<f32> {
    let air = max(sky.air_density, 0.0);
    let aerosol = max(sky.aerosols, 0.0);

    let alt = max(sky.altitude, 0.0);
    let rayleigh_density = exp(-alt / HR);
    let mie_density = exp(-alt / HM);

    let rayleigh_scale = air * rayleigh_density;
    let mie_scale = (aerosol + air * 0.05) * mie_density;

    let betaR = vec3<f32>(0.0058, 0.0135, 0.0331) * rayleigh_scale;

    let betaM = vec3<f32>(0.0200) * mie_scale * mie_len_ratio;

    let betaExt = betaR + betaM + vec3<f32>(1e-6);

    let m_sun = air_mass(saturate(sun_dir.y));
    let L_sun = m_sun * HR_KM;

    return exp(-betaExt * L_sun);
}