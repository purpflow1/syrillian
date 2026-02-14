fn luma(c: vec3f) -> f32 {
    return dot(c, vec3f(0.2126, 0.7152, 0.0722));
}

fn sample_rgb(uv: vec2f) -> vec3f {
    return textureSampleLevel(postTexture, postSampler, uv, 0.0).rgb;
}

fn fxaa(uv: vec2f) -> vec3f {
    let size  = vec2f(textureDimensions(postTexture, 0));
    let texel = 1.0 / size;

    let rgbM  = sample_rgb(uv);
    let rgbNW = sample_rgb(uv + texel * vec2f(-1.0, -1.0));
    let rgbNE = sample_rgb(uv + texel * vec2f( 1.0, -1.0));
    let rgbSW = sample_rgb(uv + texel * vec2f(-1.0,  1.0));
    let rgbSE = sample_rgb(uv + texel * vec2f( 1.0,  1.0));

    let lumaM  = luma(rgbM);
    let lumaNW = luma(rgbNW);
    let lumaNE = luma(rgbNE);
    let lumaSW = luma(rgbSW);
    let lumaSE = luma(rgbSE);

    let lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    let lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));
    let lumaRange = lumaMax - lumaMin;

    let EDGE_THRESHOLD     = 0.125;
    let EDGE_THRESHOLD_MIN = 0.0312;
    if (lumaRange < max(EDGE_THRESHOLD_MIN, lumaMax * EDGE_THRESHOLD)) {
        return rgbM;
    }

    var dir = vec2f(
        -((lumaNW + lumaNE) - (lumaSW + lumaSE)),
         ((lumaNW + lumaSW) - (lumaNE + lumaSE))
    );

    let REDUCE_MUL = 1.0 / 8.0;
    let REDUCE_MIN = 1.0 / 128.0;
    let SPAN_MAX   = 8.0;

    let dirReduce = max(
        (lumaNW + lumaNE + lumaSW + lumaSE) * (0.25 * REDUCE_MUL),
        REDUCE_MIN
    );
    let rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);

    dir = clamp(dir * rcpDirMin, vec2f(-SPAN_MAX), vec2f(SPAN_MAX)) * texel;

    let rgbA = 0.5 * (
        sample_rgb(uv + dir * (1.0 / 3.0 - 0.5)) +
        sample_rgb(uv + dir * (2.0 / 3.0 - 0.5))
    );

    let rgbB = rgbA * 0.5 + 0.25 * (
        sample_rgb(uv + dir * (-0.5)) +
        sample_rgb(uv + dir * ( 0.5))
    );

    let lumaB = luma(rgbB);
    if (lumaB < lumaMin || lumaB > lumaMax) {
        return rgbA;
    }
    return rgbB;
}

@fragment
fn fs_main(in: FInput) -> @location(0) vec4f {
    return vec4f(fxaa(in.uv), 1.0);
}
