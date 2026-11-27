shader_type sky;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform float hour = 9.0;
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
    if (ray.y > 0.026 && !AT_CUBEMAP_PASS && cloud_steps > 0) {
        vec3 sum = vec3(0.0);
        float transmission = 1.0;
        vec2 wind = drift * 0.00046 + seed_shift.xz * 0.014;
        for (int i = 0; i < 32; i++) {
            if (i >= cloud_steps || transmission < 0.018) {
                break;
            }
            float h = (float(i) + 0.5) / float(cloud_steps);
            vec2 pos = ray.xz / ray.y * (0.43 + 0.22 * h) + wind;
            float regional = noise_at(pos * 0.093, 1.0);
            float curls = noise_at(pos + regional * vec2(0.27, -0.39) + h * vec2(0.12, 0.28), 0.0);
            float body = 1.0 - pow(abs(2.0 * h - 1.0), 2.4);
            float cloud = smoothstep(0.49, 0.75, curls + regional * 0.37 + cloudiness * 0.53 - 0.34) * body;
            float extinction = 1.0 - exp(-cloud * 7.9 / max(0.39, ray.y) / float(cloud_steps));
            vec3 shade = mix(vec3(0.076, 0.10, 0.15), vec3(0.82, 0.88, 0.91), daytime * (0.36 + 0.59 * h));
            shade += twilight * facing * vec3(0.31, 0.18, 0.08);
            sum += transmission * shade * extinction;
            transmission *= 1.0 - extinction;
        }
        color = mix(color, sum + color * transmission, smoothstep(0.026, 0.086, ray.y));
    }
    COLOR = color * (1.0 - 0.57 * clamp(cloudiness - 0.47, 0.0, 0.5));
}
