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

vec3 relief_normal(vec3 normal, vec3 position, float height) {
    vec3 dx = dFdx(position);
    vec3 dy = dFdy(position);
    vec3 along_x = cross(dy, normal);
    vec3 along_y = cross(normal, dx);
    float determinant = dot(dx, along_x);
    vec3 gradient = (dFdx(height) * along_x + dFdy(height) * along_y)
        * sign(determinant) / max(abs(determinant), 0.000001);
    return normalize(normal - gradient * inversesqrt(1.0 + dot(gradient, gradient)));
}

void fragment() {
    vec3 p = site + seed_shift;
    vec3 face = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    vec3 base = COLOR.rgb;
    float climate_snow = surface_type == 1 ? 0.0 : COLOR.a;
    float steepness = 1.0 - max(0.0, face.y);
    float exposure = grain(p.xz * 0.0027 + vec2(p.y * 0.0034, 0.0));
    float coarse = grain(p.xz * 0.034 + vec2(exposure, -exposure) * 0.54);
    float snow = max(climate_snow, snowfall)
        * smoothstep(0.37, 0.89, face.y + 0.72 * (coarse - 0.5));
    vec2 bedding = vec2(p.x * 0.58 + p.z * 0.79, p.y + p.x * 0.11 - p.z * 0.15);

    if (surface_type == 2) {
        // Divided rock beds catch light between gullies at scales the mesh cannot resolve.
        float layer = grain(bedding * vec2(0.014, 0.065) + vec2(exposure, -exposure) * 0.33);
        float gully = grain(vec2(bedding.x * 0.055, p.y * 0.009) + vec2(coarse * 0.22, exposure));
        float cleft = 1.0 - smoothstep(0.047, 0.23, abs(gully - 0.47) + (coarse - 0.45) * 0.28);
        float ledge = smoothstep(0.33, 0.69, layer + (coarse - 0.5) * 0.38);
        float rock_face = smoothstep(0.14, 0.48, steepness + (exposure - 0.48) * 0.20);
        float crowns = grain(p.xz * 0.16 + vec2(exposure, -coarse) * 0.51);
        float cover = smoothstep(0.09, 0.59, UV.x * (0.35 + coarse * 0.85 + crowns * 0.48))
            * (1.0 - rock_face * 0.87);
        vec3 forest = mix(vec3(0.025, 0.049, 0.024), vec3(0.083, 0.112, 0.045), crowns);
        vec3 land = base * (0.51 + 0.41 * coarse + 0.34 * exposure);
        vec3 rock = mix(vec3(0.082, 0.073, 0.062), vec3(0.29, 0.28, 0.25), ledge)
            * (0.96 + 0.26 * exposure - 0.27 * cleft);
        float drift = smoothstep(0.52, 0.87, face.y * 0.67 + cleft * 0.25 + ledge * 0.14);
        snow = max(snow, climate_snow * drift);
        ALBEDO = mix(mix(mix(land, rock, rock_face), forest, cover), vec3(0.70, 0.76, 0.82), snow);
        ROUGHNESS = mix(0.89, 0.70 + ledge * 0.18, rock_face);
        AO = 1.0 - cleft * rock_face * (1.0 - snow) * 0.24;
        float height = (1.25 * ledge - 0.92 * cleft) * rock_face * (1.0 - snow * 0.85);
        NORMAL = relief_normal(NORMAL, VERTEX, height * relief);
    } else {
        // Two rotated footprints with independent warp phases keep the shared field
        // from printing a regular tile. Thickness is resolved in world metres.
        vec2 fine_uv = mat2(vec2(0.76, 0.65), vec2(-0.65, 0.76)) * p.xz * 0.35
            + vec2(exposure, -coarse) * 0.63;
        float fine = grain(fine_uv);
        float footprint = max(length(dFdx(p)), length(dFdy(p)));
        float detail = 1.0 - smoothstep(0.032, 0.13, footprint);
        float grit_field = grain(p.xz * 1.65 + vec2(coarse, exposure) * 0.34);
        float gravel = smoothstep(0.56, 0.74, fine + 0.25 * grit_field) * detail;

        float rock = max(smoothstep(0.12, 0.49, steepness + (coarse - 0.5) * 0.16), float(surface_type == 1));
        float strata = coarse;
        vec2 stone_uv = bedding * vec2(0.074, 0.21) + vec2(exposure * 0.73, coarse * 0.37);
        vec2 stone_dx = dFdx(stone_uv);
        vec2 stone_dy = dFdy(stone_uv);
        if (rock > 0.01) {
            strata = textureGrad(ground_field, stone_uv, stone_dx, stone_dy).r;
        }
        float fracture = 1.0 - smoothstep(0.027, 0.17, abs(strata - 0.43 + (fine - 0.5) * 0.16));
        float plate = smoothstep(0.31, 0.72, strata + (coarse - 0.5) * 0.53);
        float sand = UV.x * float(surface_type == 0);
        float ash = UV.y * float(surface_type == 0);
        float green = smoothstep(0.008, 0.115, base.g - base.r);
        float duff = green * smoothstep(0.33, 0.68, coarse + (exposure - 0.5) * 0.47);
        vec3 topsoil = base * (0.47 + 0.31 * coarse + 0.37 * exposure + 0.13 * fine);
        vec3 litter = mix(vec3(0.045, 0.031, 0.018), vec3(0.144, 0.095, 0.052), fine);
        vec3 soil = mix(topsoil, litter, duff * 0.83);
        if (surface_type == 3) {
            soil = base * (0.61 + 0.43 * fine + 0.29 * grit_field);
        }
        soil = mix(soil, vec3(0.21, 0.177, 0.134) * (0.63 + coarse), gravel * (0.55 - duff * 0.37));

        float dune_ripple = sin(dot(p.xz, vec2(9.0, -14.6)) + 9.0 * coarse + 5.5 * exposure);
        vec3 quartz = vec3(0.46, 0.325, 0.188)
            * (0.57 + 0.34 * exposure + 0.24 * coarse + 0.044 * dune_ripple * detail);
        soil = mix(soil, quartz, sand * 0.83);

        vec3 rock_color = mix(vec3(0.095, 0.084, 0.072), vec3(0.31, 0.28, 0.239), plate);
        rock_color *= 0.72 + 0.42 * exposure + 0.14 * fine - 0.27 * fracture;
        rock_color = mix(rock_color, vec3(0.31, 0.205, 0.134) * (0.58 + 0.71 * strata), sand * 0.65);
        vec3 basalt = vec3(0.039, 0.032, 0.029) * (0.60 + 0.62 * plate + fine * 0.24 - 0.23 * fracture);
        vec3 pigment = mix(mix(soil, rock_color, rock), basalt, ash * 0.92);
        float wet = wetness * (1.0 - snow) * (0.46 + fine * 0.51);
        pigment = mix(pigment, pigment * vec3(0.57, 0.64, 0.67), wet);
        pigment = mix(pigment, vec3(0.43, 0.27, 0.13), grit * 0.19 * (1.0 - snow));
        ALBEDO = mix(pigment, vec3(0.71, 0.77, 0.83) * (0.82 + 0.28 * fine), snow);
        float dry_roughness = mix(0.89 + gravel * 0.06, 0.64 + 0.22 * fracture + 0.06 * fine, rock);
        ROUGHNESS = mix(mix(dry_roughness, mix(0.49, 0.34, rock), wet), 0.86, snow);
        SPECULAR = mix(0.39, 0.50, wet);
        AO = mix(0.85 + 0.15 * fine, 1.0 - 0.31 * fracture, rock);

        float soil_height = fine * 0.012 + gravel * 0.009
            + detail * (grit_field * 0.0017 + sand * dune_ripple * 0.0015);
        float stone_height = 0.052 * plate - 0.028 * fracture + 0.016 * fine;
        float height = mix(soil_height, stone_height, rock) * mix(1.0, 0.24, snow);
        NORMAL = relief_normal(NORMAL, VERTEX, height * relief);
    }
}
