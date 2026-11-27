shader_type sky;
render_mode use_half_res_pass;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform float cloudiness = 0.42;
uniform vec2 drift;
uniform int cloud_steps = 20;

float noise_at(vec2 p, float lod) {
    return textureLod(ground_field, p, lod).r;
}

void sky() {
    vec3 ray = EYEDIR;
    float altitude = max(ray.y, 0.0);
    vec3 sun_dir = LIGHT0_DIRECTION;
    float elevation = sun_dir.y;
    float daytime = smoothstep(-0.14, 0.22, elevation);
    float horizon = exp(-altitude * 7.2);
    float facing = max(0.0, dot(ray, sun_dir));
    float twilight = exp(-abs(elevation) * 6.25) * daytime;
    vec3 zenith = mix(vec3(0.007, 0.014, 0.032), vec3(0.16, 0.36, 0.70), daytime);
    vec3 band = mix(vec3(0.014, 0.024, 0.043), vec3(0.63, 0.76, 0.89), daytime);
    band = mix(band, vec3(0.91, 0.42, 0.17), twilight * (0.30 + 0.48 * facing));
    vec3 color = mix(zenith, band, horizon);
    color += vec3(1.0, 0.77, 0.49) * pow(facing, 58.0) * daytime * 0.43;
    if (ray.y > 0.0 && elevation > -0.03) {
        color += smoothstep(0.99979, 0.99993, facing) * daytime * vec3(7.0, 5.3, 3.4);
    }
    if (elevation < -0.08 && ray.y > 0.0) {
        float celestial = (1.0 - daytime) * smoothstep(0.0, 0.12, altitude);
        vec2 stars = (ray.xz + seed_shift.xy * 0.003) / (1.0 + altitude) * 218.0;
        vec2 cell = floor(stars);
        float star = fract(sin(dot(cell, vec2(41.63, 87.13))) * 37179.173);
        float spark = 1.0 - smoothstep(0.03, 0.22, length(fract(stars) - vec2(0.5)));
        color += spark * smoothstep(0.948, 0.997, star) * celestial;
        vec3 moon = normalize(vec3(-sun_dir.x, -sun_dir.y + 0.1, -sun_dir.z + 0.25));
        color += vec3(0.71, 0.83, 1.0) * smoothstep(0.99975, 0.99986, dot(ray, moon)) * celestial;
    }
    vec2 plane = ray.xz / max(0.025, ray.y);
    vec2 phase = seed_shift.xz * 0.014;
    float fade = smoothstep(0.026, 0.14, ray.y);
    if (AT_HALF_RES_PASS) {
        vec3 sum = vec3(0.0);
        float transmission = 1.0;
        if (fade > 0.0 && !AT_CUBEMAP_PASS) {
            vec2 wind = drift * 0.0004 + phase;
            float regional = noise_at(plane * 0.061 + phase * vec2(-0.37, 0.67), 1.0);
            float front = smoothstep(0.27, 0.77, regional + (cloudiness - 0.37) * 0.61);
            float step_size = 1.0 / float(max(1, cloud_steps));
            for (int i = 0; i < 32; i++) {
                if (i >= cloud_steps || transmission < 0.018) {
                    break;
                }
                float h = (float(i) + 0.5) * step_size;
                vec2 pos = plane * (0.36 + 0.32 * h) + wind;
                float banks = noise_at(pos * 0.27 + phase.yx * 0.76, 1.0);
                vec2 curl_uv = mat2(vec2(0.87, -0.49), vec2(0.49, 0.87)) * pos + banks * vec2(0.36, -0.42);
                float curls = noise_at(curl_uv + h * vec2(0.18, 0.24), 0.0);
                float body = smoothstep(0.0, 0.28, h) * (1.0 - smoothstep(0.52 + banks * 0.26, 0.99, h));
                float cloud = smoothstep(0.69, 0.98, curls * 0.52 + banks * 0.35 + front * 0.48) * body;
                float extinction = 1.0 - exp(-cloud * 7.7 * step_size / max(0.34, ray.y));
                vec3 shade = mix(vec3(0.058, 0.084, 0.13), vec3(0.69, 0.75, 0.81), daytime * (0.26 + 0.73 * h));
                vec3 silver = vec3(1.0, 0.76, 0.55) * (0.53 + twilight * 0.40);
                shade += silver * pow(facing, 13.0) * daytime * (0.38 + 0.67 * (1.0 - cloud));
                sum += transmission * shade * extinction;
                transmission *= 1.0 - extinction;
            }
        }
        COLOR = sum * fade;
        ALPHA = (1.0 - transmission) * fade;
    } else {
        float streaks = noise_at(plane * vec2(0.13, 0.28) + phase * 0.57 + drift * 0.00012, 1.0);
        float cirrus = smoothstep(0.56, 0.79, streaks) * fade * (0.23 + 0.18 * cloudiness);
        color = mix(color, vec3(0.65, 0.72, 0.80) * (0.15 + 0.85 * daytime), cirrus);
        if (!AT_CUBEMAP_PASS) {
            color = color * (1.0 - HALF_RES_COLOR.a) + HALF_RES_COLOR.rgb;
        }
        COLOR = color * (1.0 - 0.57 * clamp(cloudiness - 0.47, 0.0, 0.5));
    }
}
