struct FInput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@group(1) @binding(0)
var postTexture: texture_2d<f32>;
@group(1) @binding(1)
var postSampler: sampler;
@group(1) @binding(2)
var postDepth: texture_depth_2d;
@group(1) @binding(3)
var postNormal: texture_2d<f32>;
@group(1) @binding(4)
var postMaterial: texture_2d<f32>;

fn post_rrt_and_odt_fit(v: vec3f) -> vec3f {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn post_tonemap_aces(color: vec3f) -> vec3f {
    let aces_in = mat3x3<f32>(
        vec3f(0.59719, 0.07600, 0.02840),
        vec3f(0.35458, 0.90834, 0.13383),
        vec3f(0.04823, 0.01566, 0.83777),
    );
    let aces_out = mat3x3<f32>(
        vec3f(1.60475, -0.10208, -0.00327),
        vec3f(-0.53108, 1.10813, -0.07276),
        vec3f(-0.07367, -0.00605, 1.07602),
    );

    let v = aces_in * color;
    let r = post_rrt_and_odt_fit(v);
    let o = aces_out * r;
    return clamp(o, vec3f(0.0), vec3f(1.0));
}

fn post_color_grade(color: vec4f) -> vec4f {
    let graded = post_tonemap_aces(max(color.rgb, vec3f(0.0)));
    return vec4f(graded, color.a);
}
