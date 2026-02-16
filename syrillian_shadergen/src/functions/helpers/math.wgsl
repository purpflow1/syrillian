const PI: f32 = 3.14159265359;
const RAD_TO_DEG: f32 = 57.29577951308232;

fn sum4(v: vec4<f32>) -> f32 { return v.x + v.y + v.z + v.w; }
fn safe_rsqrt(x: f32) -> f32 { return inverseSqrt(max(x, 1e-8)); }
fn safe_normalize(v: vec3<f32>) -> vec3<f32> { return v * safe_rsqrt(dot(v, v)); }

fn sign_not_zero_1(x: f32) -> f32 {
    return select(-1.0, 1.0, x >= 0.0);
}

fn sign_not_zero_2(v: vec2f) -> vec2f {
    return vec2f(sign_not_zero_1(v.x), sign_not_zero_1(v.y));
}

fn oct_encode(n_in: vec3f) -> vec2f {
    let n = normalize(n_in);
    let denom = max(abs(n.x) + abs(n.y) + abs(n.z), 1e-6);
    var v = n / denom;
    var enc = v.xy;

    if (v.z < 0.0) {
        enc = (1.0 - abs(enc.yx)) * sign_not_zero_2(enc);
    }
    return enc;
}

fn oct_decode(enc_in: vec2f) -> vec3f {
    let enc = clamp(enc_in, vec2f(-1.0), vec2f(1.0));
    var v = vec3f(enc.x, enc.y, 1.0 - abs(enc.x) - abs(enc.y));
    if (v.z < 0.0) {
        let new_v = (1.0 - abs(v.yx)) * sign_not_zero_2(v.xy);
        v.x = new_v.x;
        v.y = new_v.y;
    }
    return normalize(v);
}
