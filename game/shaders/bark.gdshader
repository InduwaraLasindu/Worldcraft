shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;
#include "res://shaders/tree_motion.gdshaderinc"
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 seed_shift;
uniform float wetness;
uniform float kind;
uniform float tree_height;
uniform vec2 wind;
uniform float clock;
uniform float flex = 1.0;
varying vec4 variation;

void vertex() {
    variation = INSTANCE_CUSTOM;
    if (tree_height > 0.0) {
        VERTEX = move_tree(VERTEX, MODEL_MATRIX, variation, tree_height, wind, clock, flex);
    }
}

void fragment() {
    vec2 uv = UV + vec2(variation.r, variation.g) + seed_shift.xz * 0.008;
    float coarse = texture(ground_field, uv * vec2(0.59, 0.11)).r;
    float flute = texture(ground_field, uv * vec2(2.32, 0.31) + vec2(coarse * 0.36, 0.0)).r;
    float fine = texture(ground_field, uv * vec2(4.23, 3.70)).r;
    float crevice = smoothstep(0.31, 0.63, flute + fine * 0.17);
    // Wrapped longitudinal fissures and separated bark plates.
    float angle = UV.x * 2.0943951;
    vec2 wrapped = vec2(cos(angle), sin(angle));
    float seams = texture(ground_field, wrapped * 0.86 + vec2(uv.y * 0.071, uv.y * 0.043) + uv.x * 0.023).r;
    float grain = smoothstep(0.25, 0.66, flute * 0.65 + seams * 0.47 + (fine - 0.5) * 0.21);
    vec3 wood = mix(vec3(0.031, 0.023, 0.016), vec3(0.161, 0.132, 0.098), grain);
    if (kind == 1.0) {
        float dark = texture(ground_field, uv * vec2(0.64, 2.9)).r;
        float lenticels = smoothstep(0.63, 0.76, dark + 0.12 * seams);
        float scars = smoothstep(0.60, 0.83, coarse + fine * 0.12);
        wood = mix(vec3(0.53, 0.49, 0.41), vec3(0.065, 0.049, 0.037), max(lenticels, scars));
    } else if (kind == 2.0) {
        wood = mix(vec3(0.036, 0.023, 0.014), vec3(0.177, 0.099, 0.061), grain);
    } else if (kind == 3.0) {
        float ring = sin(UV.y * 34.0 + seams * 1.9);
        float old_frond = smoothstep(0.12, 0.86, ring + coarse * 0.24);
        wood = mix(vec3(0.085, 0.069, 0.046), vec3(0.34, 0.30, 0.233), old_frond * 0.64 + fine * 0.27);
        grain = old_frond * 0.62 + fine * 0.27;
    } else if (kind == 4.0) {
        wood = mix(vec3(0.044, 0.039, 0.032), vec3(0.15, 0.134, 0.101), grain);
    }
    wood = mix(wood, vec3(0.077, 0.085, 0.046), smoothstep(0.63, 0.92, coarse + fine * 0.13) * 0.46);
    if (COLOR.a < 0.5) {
        float ring = smoothstep(-0.47, 0.23, sin(length(UV - vec2(0.52, 0.47)) * 91.0 + fine * 3.1));
        wood = mix(vec3(0.18, 0.114, 0.070), vec3(0.44, 0.29, 0.168), ring);
    }
    ALBEDO = wood * (0.66 + fine * 0.63) * (0.80 + 0.26 * variation.b) * (1.0 - wetness * 0.38);
    ROUGHNESS = mix(0.87 + 0.08 * (1.0 - crevice), 0.49, wetness);
    vec3 x_grad = cross(dFdy(VERTEX), NORMAL);
    vec3 y_grad = cross(NORMAL, dFdx(VERTEX));
    float det = dot(dFdx(VERTEX), x_grad);
    if (abs(det) > 0.000001) {
        NORMAL = normalize(NORMAL - (dFdx(grain) * x_grad + dFdy(grain) * y_grad) * (0.044 / det));
    }
}
