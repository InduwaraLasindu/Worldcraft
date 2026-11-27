shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 seed_shift;
uniform float wetness;
uniform float kind;
varying vec4 variation;

void vertex() {
    variation = INSTANCE_CUSTOM;
}

void fragment() {
    vec2 uv = UV + vec2(variation.r, variation.g) + seed_shift.xz * 0.008;
    float coarse = texture(ground_field, uv * vec2(0.59, 0.11)).r;
    float flute = texture(ground_field, uv * vec2(2.32, 0.31) + vec2(coarse * 0.36, 0.0)).r;
    float fine = texture(ground_field, uv * vec2(4.23, 3.70)).r;
    float crevice = smoothstep(0.31, 0.63, flute + fine * 0.17);
    vec3 wood = mix(vec3(0.028, 0.019, 0.013), vec3(0.154, 0.118, 0.080), crevice);
    float bark_paper = smoothstep(0.20, 0.55, coarse + 0.04 * sin(uv.y * 19.0));
    vec3 birch = mix(vec3(0.045, 0.036, 0.031), vec3(0.57, 0.50, 0.41), bark_paper);
    wood = mix(wood, birch, step(0.5, kind));
    wood = mix(wood, vec3(0.10, 0.128, 0.062), smoothstep(0.61, 0.96, coarse) * 0.36);
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
        NORMAL = normalize(NORMAL - (dFdx(flute) * x_grad + dFdy(flute) * y_grad) * (0.061 / det));
    }
}
