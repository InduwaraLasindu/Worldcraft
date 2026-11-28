shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 seed_shift;
uniform vec3 world_shift;
uniform float wetness;
uniform float snowfall;
uniform float grit;
uniform int surface_type; // 0 terrain, 1 boulders, 2 distant terrain, 3 moss
uniform float relief = 1.0;
varying vec3 site;

void vertex() {
    site = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz + world_shift;
}

float grain(vec2 p) {
    return texture(ground_field, p).r;
}

void fragment() {
    vec3 p = site + seed_shift;
    vec3 face = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    float slope = smoothstep(0.14, 0.53, 1.0 - max(0.0, face.y));
    vec3 base = COLOR.rgb;
    float climate_snow = surface_type == 1 ? 0.0 : COLOR.a;
    float exposure = grain(p.xz * 0.008 + vec2(p.y * 0.011, 0.0));
    float coarse = grain(p.xz * 0.037 + vec2(exposure, -exposure) * 0.73);
    float snow = max(climate_snow, snowfall) * smoothstep(0.37, 0.89, face.y + (coarse - 0.5) * 0.48);

    if (surface_type == 2) {
        // Climate canopy weights drive the visible forest beyond the real tree batches.
        float crowns = grain(p.xz * 0.16 + vec2(exposure, -coarse) * 0.51);
        float cover = smoothstep(0.11, 0.67, UV.x * (0.45 + 0.88 * coarse + 0.40 * crowns));
        vec3 forest = mix(vec3(0.026, 0.058, 0.036), vec3(0.074, 0.117, 0.060), crowns);
        vec3 land = base * (0.57 + 0.35 * coarse + 0.32 * exposure);
        vec2 face_uv = vec2(p.x - p.z * 0.75, p.y + p.z * 0.46) * vec2(0.047, 0.117);
        float strata = grain(face_uv + vec2(coarse * 0.49, exposure * 0.83));
        float rock_face = smoothstep(0.19, 0.56, (1.0 - face.y) + 0.18 * coarse);
        vec3 exposed = mix(vec3(0.091, 0.077, 0.064), vec3(0.42, 0.38, 0.34), smoothstep(0.16, 0.75, strata));
        land = mix(land, exposed * (0.71 + 0.34 * exposure), rock_face * (1.0 - cover * 0.65));
        float drift_snow = max(snow, climate_snow * smoothstep(0.58, 0.92, 0.60 * strata + face.y * 0.53));
        ALBEDO = mix(mix(land, forest, cover), vec3(0.69, 0.75, 0.81), drift_snow);
        ROUGHNESS = 0.86;
    } else {
        // Two rotated footprints with independent warp phases keep the shared field
        // from printing a regular tile. Thickness is resolved in world metres.
        vec2 fine_uv = mat2(vec2(0.76, 0.65), vec2(-0.65, 0.76)) * p.xz * 0.35
            + vec2(exposure, -coarse) * 0.63;
        float fine = grain(fine_uv);
        float grit_field = grain(p.xz * 1.75 + vec2(coarse, exposure) * 0.34);
        float footprint = length(fwidth(p));
        float detail = 1.0 - smoothstep(0.095, 0.31, footprint);
        float gravel = smoothstep(0.60, 0.74, fine + 0.17 * grit_field) * detail;

        float rock = max(slope, float(surface_type == 1));
        float strata = coarse;
        vec2 bedding = vec2(p.x * 0.58 + p.z * 0.79, p.y + p.x * 0.11 - p.z * 0.15);
        vec2 stone_uv = bedding * vec2(0.074, 0.21) + vec2(exposure * 0.73, coarse * 0.37);
        vec2 stone_dx = dFdx(stone_uv);
        vec2 stone_dy = dFdy(stone_uv);
        if (rock > 0.01) {
            strata = textureGrad(ground_field, stone_uv, stone_dx, stone_dy).r;
        }
        float fracture = smoothstep(0.14, 0.46, abs(strata - 0.43) + grit_field * 0.16);
        float sand = UV.x * float(surface_type == 0);
        float ash = UV.y * float(surface_type == 0);
        float green = smoothstep(0.008, 0.115, base.g - base.r);
        float duff = green * smoothstep(0.32, 0.73, coarse + 0.12 * exposure);
        vec3 topsoil = base * (0.52 + 0.32 * fine + 0.36 * exposure);
        vec3 litter = mix(vec3(0.048, 0.033, 0.021), vec3(0.13, 0.092, 0.048), fine);
        vec3 soil = mix(topsoil, litter, duff * 0.74);
        if (surface_type == 3) {
            soil = base * (0.61 + 0.43 * fine + 0.29 * grit_field);
        }
        soil = mix(soil, vec3(0.23, 0.195, 0.153) * (0.65 + coarse), gravel * 0.45);

        float dune_ripple = sin(dot(p.xz, vec2(9.0, -14.6)) + 9.0 * coarse + 5.5 * exposure);
        vec3 quartz = vec3(0.47, 0.325, 0.179) * (0.61 + 0.27 * exposure + 0.32 * fine + 0.055 * dune_ripple * detail);
        soil = mix(soil, quartz, sand * 0.83);

        vec3 rock_color = mix(vec3(0.112, 0.099, 0.084), vec3(0.32, 0.30, 0.267), fracture);
        rock_color *= 0.73 + 0.36 * exposure + 0.28 * fine;
        rock_color = mix(rock_color, vec3(0.31, 0.205, 0.134) * (0.58 + 0.71 * strata), sand * 0.65);
        vec3 basalt = vec3(0.039, 0.032, 0.029) * (0.50 + 0.84 * fracture + fine * 0.33);
        vec3 pigment = mix(mix(soil, rock_color, rock), basalt, ash * 0.92);
        float wet = wetness * (1.0 - snow) * (0.46 + fine * 0.51);
        pigment = mix(pigment, pigment * vec3(0.57, 0.64, 0.67), wet);
        pigment = mix(pigment, vec3(0.43, 0.27, 0.13), grit * 0.19 * (1.0 - snow));
        ALBEDO = mix(pigment, vec3(0.71, 0.77, 0.83) * (0.82 + 0.28 * fine), snow);
        ROUGHNESS = mix(mix(0.87 - 0.18 * rock + (fine - 0.5) * 0.09, 0.34, wet), 0.86, snow);
        SPECULAR = mix(0.34, 0.55, wet);
        AO = mix(0.78 + 0.22 * fine, 0.66 + 0.34 * fracture, rock);
        AO_LIGHT_AFFECT = 0.23;

        float soil_height = fine * 0.59 + gravel * 0.24 + grit_field * 0.22 + sand * dune_ripple * detail * 0.07;
        float stone_height = 0.79 * strata + 0.28 * fracture + 0.35 * fine;
        float micro = mix(soil_height, stone_height, rock) * mix(1.0, 0.35, snow);
        vec3 sx = dFdx(VERTEX);
        vec3 sy = dFdy(VERTEX);
        vec3 x_grad = cross(sy, NORMAL);
        vec3 y_grad = cross(NORMAL, sx);
        float determinant = dot(sx, x_grad);
        if (abs(determinant) > 0.000001) {
            vec3 delta = (dFdx(micro) * x_grad + dFdy(micro) * y_grad) / determinant;
            NORMAL = normalize(NORMAL - delta * relief * mix(0.027, 0.092, rock));
        }
    }
}
