shader_type sky;
render_mode use_half_res_pass;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform float cloudiness = 0.38;
uniform vec2 drift;
uniform int cloud_steps = 20;

float noise_at(vec2 p, float lod) {
    return textureLod(ground_field, p, lod).r;
}

// Curvature bounds the optical path even at the horizon.
float air_mass(float elevation) {
    float up = clamp(elevation, 0.0, 1.0);
    return 1.0 / (up + 0.088 * sqrt(1.0 - up));
}

float aerosol_phase(float angle, float g) {
    float separation = 1.0 + g * g - 2.0 * g * angle;
    return (1.0 - g * g) * inversesqrt(separation) / separation;
}

vec3 atmosphere(float altitude, float sun_height, float angle, float daylight, vec3 sunlight) {
    float path = air_mass(altitude);
    float rayleigh = 0.75 * (1.0 + angle * angle);
    vec3 scatter = 1.0 - exp(-vec3(0.032, 0.109, 0.295) * path);
    vec3 blue = scatter * rayleigh * (0.59 + 0.14 * clamp(sun_height, 0.0, 1.0));
    float distance_haze = 1.0 - exp(-0.145 * path);
    float forward = min(5.6, aerosol_phase(angle, 0.77));
    vec3 haze = mix(vec3(0.52, 0.62, 0.72), sunlight, 0.46) * distance_haze * (0.21 + 0.15 * forward);
    float afterglow = exp(-abs(sun_height) * 13.0) * exp(-altitude * 8.1);
    vec3 dusk = vec3(0.67, 0.235, 0.095) * afterglow * (0.10 + 0.67 * pow(max(0.0, angle), 3.0));
    vec3 dark = mix(vec3(0.013, 0.022, 0.044), vec3(0.0018, 0.0039, 0.0092), smoothstep(0.0, 0.37, altitude));
    return mix(dark, blue + haze, daylight) + dusk;
}

float cloud_mass(vec2 p, float h, vec2 seed, float lod) {
    return noise_at(p * 0.043 + seed + vec2(0.009, -0.005) * h, lod);
}

float cloud_profile(float h) {
    return smoothstep(0.015, 0.19, h) * (1.0 - smoothstep(0.66, 0.99, h));
}

float cloud_edge(float bulk, float h, float threshold) {
    float above_base = h - 0.27;
    return bulk - threshold - above_base * above_base * 0.29;
}

void sky() {
    vec3 ray = EYEDIR;
    vec3 sun_dir = LIGHT0_DIRECTION;
    float altitude = max(ray.y, 0.0);
    float sun_height = sun_dir.y;
    float daytime = smoothstep(-0.12, 0.21, sun_height);
    float angle = clamp(dot(ray, sun_dir), -1.0, 1.0);
    vec3 sunlight = mix(vec3(1.0, 0.45, 0.23), LIGHT0_COLOR, smoothstep(-0.035, 0.38, sun_height));
    float overcast = smoothstep(0.38, 1.0, cloudiness);
    vec2 seed = seed_shift.xz * 0.014;
    vec2 wind = drift * 0.0004;
    vec2 plane = ray.xz / max(0.025, altitude);
    float fade = smoothstep(0.037, 0.19, ray.y);

    if (AT_HALF_RES_PASS) {
        vec3 sum = vec3(0.0);
        float transmission = 1.0;
        if (fade > 0.0 && !AT_CUBEMAP_PASS) {
            float regional = noise_at(plane * 0.011 + seed.yx * vec2(-0.49, 0.82) + wind * 0.012, 1.0);
            float threshold = mix(0.625, 0.347, overcast) + (0.48 - regional) * 0.20;
            float phase = min(4.5, aerosol_phase(angle, 0.66));
            float step_size = 1.0 / float(max(1, cloud_steps));
            float span = min(0.24, altitude * 1.15);
            float footprint = log2(1.0 + length(plane) * span * step_size * 34.0);
            float texture_lod = log2(float(textureSize(ground_field, 0).x) / 1024.0);
            float bulk_lod = max(0.0, texture_lod + 0.65 + footprint);
            float detail_lod = max(0.0, texture_lod + 0.78 + 1.55 * footprint);
            // Offset the strata on nearby rays to soften sampling bands near the horizon.
            vec2 pixel = floor(SCREEN_UV * vec2(1987.0, 1123.0));
            float sample_noise = fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
            float sample_offset = 0.5 + (sample_noise - 0.5) * mix(0.62, 0.16, smoothstep(0.11, 0.63, altitude));
            float step_extinction = step_size * 10.5 / max(0.36, altitude);
            vec3 cloud_light = sunlight * smoothstep(-0.055, 0.18, sun_height) * mix(1.0, 0.53, overcast);
            vec3 ambient = mix(vec3(0.016, 0.022, 0.037), vec3(0.23, 0.29, 0.37), daytime);
            vec2 detail_seed = seed.yx * 1.53 + vec2(0.64, -0.31);
            mat2 detail_basis = mat2(vec2(0.86, -0.51), vec2(0.51, 0.86));
            for (int i = 0; i < 32; i++) {
                if (i >= cloud_steps || transmission < 0.018) {
                    break;
                }
                float h = (float(i) + sample_offset) * step_size;
                vec2 position = plane * (0.88 + span * h) + wind;
                float bulk = cloud_mass(position, h, seed, bulk_lod);
                float edge = cloud_edge(bulk, h, threshold);
                if (edge > -0.119) {
                    float billows = noise_at(detail_basis * position * 0.155 + detail_seed + h * vec2(0.039, 0.028), detail_lod);
                    float density = max(0.0, edge + (billows - 0.53) * 0.25) * cloud_profile(h);
                    if (density > 0.0) {
                        float ahead_h = h + 0.19 * sun_dir.y;
                        vec2 ahead_pos = position + sun_dir.xz * 0.26;
                        float ahead = max(0.0, cloud_edge(cloud_mass(ahead_pos, ahead_h, seed, bulk_lod), ahead_h, threshold));
                        float sun_depth = density * 6.4 + ahead * cloud_profile(ahead_h) * 51.0;
                        float direct = 1.0 / (1.0 + sun_depth + sun_depth * sun_depth * 0.14);
                        vec3 lit = ambient * mix(0.62, 1.16, h) + cloud_light * (0.12 + direct * (1.10 + 0.17 * phase));
                        float extinction = 1.0 - exp(-density * step_extinction * 10.0);
                        sum += transmission * lit * extinction;
                        transmission *= 1.0 - extinction;
                    }
                }
            }
        }
        float opacity = (1.0 - transmission) * fade;
        float horizon_haze = 0.46 * exp(-altitude * 8.0);
        COLOR = mix(sum * fade, atmosphere(altitude, sun_height, angle, daytime, sunlight) * opacity, horizon_haze);
        ALPHA = opacity;
    } else {
        vec3 color = atmosphere(altitude, sun_height, angle, daytime, sunlight);
        color = mix(color, mix(vec3(0.012, 0.019, 0.030), vec3(0.35, 0.39, 0.45), daytime), 0.26 * overcast);
        if (ray.y > 0.0 && sun_height > -0.014 && !AT_CUBEMAP_PASS) {
            // The apparent solar diameter is about half a degree.
            float disc = smoothstep(0.9999867, 0.9999920, angle);
            float halo = 0.35 * exp(-2200.0 * (1.0 - angle));
            color += sunlight * (disc * 12.0 + halo) * smoothstep(-0.014, 0.026, sun_height) * (1.0 - overcast * 0.78);
        }
        if (sun_height < -0.08 && ray.y > 0.0) {
            float celestial = (1.0 - daytime) * smoothstep(0.0, 0.12, altitude) * (1.0 - 0.85 * overcast);
            vec2 stars = (ray.xz + seed_shift.xy * 0.003) / (1.0 + altitude) * 218.0;
            float star = fract(sin(dot(floor(stars), vec2(41.63, 87.13))) * 37179.173);
            float spark = 1.0 - smoothstep(0.03, 0.22, length(fract(stars) - vec2(0.5)));
            color += spark * smoothstep(0.948, 0.997, star) * celestial;
            vec3 moon = normalize(vec3(-sun_dir.x, -sun_dir.y + 0.10, -sun_dir.z + 0.25));
            color += vec3(0.71, 0.83, 1.0) * smoothstep(0.999965, 0.999981, dot(ray, moon)) * celestial;
        }
        if (!AT_CUBEMAP_PASS) {
            vec2 feather = vec2(plane.x * 0.041 + plane.y * 0.087, plane.y * 0.13 - plane.x * 0.022);
            float streaks = noise_at(feather + seed * 1.34 + wind * 0.24, 1.6);
            float cirrus = smoothstep(0.64, 0.83, streaks) * fade * (0.13 + 0.11 * overcast);
            color = mix(color, vec3(0.63, 0.68, 0.76) * (0.07 + 0.93 * daytime), cirrus);
            color = color * (1.0 - HALF_RES_COLOR.a) + HALF_RES_COLOR.rgb;
        }
        COLOR = color;
    }
}
