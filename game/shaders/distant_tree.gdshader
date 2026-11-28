shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_disabled;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform bool conifer;
varying vec4 variation;

void vertex() {
    variation = INSTANCE_CUSTOM;
}

void fragment() {
    vec2 p = UV * vec2(2.0, 1.0) - vec2(1.0, 0.0);
    vec2 phase = variation.rg + seed_shift.xz * 0.021;
    float bough = texture(ground_field, UV * vec2(0.61, 0.88) + phase).r;
    float twigs = texture(ground_field, UV * vec2(2.4, 3.8) + phase.yx * 1.73).r;
    float center = (variation.r - 0.5) * p.y * 0.22;
    float x = abs(p.x + center);
    float crown;
    if (conifer) {
        float layer = fract(p.y * (10.7 + variation.g * 4.9) + x * 0.94 + bough * 0.57);
        float tier = mix(0.45, 1.13, smoothstep(0.05, 0.67, layer) * (1.0 - smoothstep(0.81, 1.0, layer)));
        float extent = pow(max(0.0, 1.0 - p.y), 0.94) * smoothstep(0.15, 0.25, p.y);
        crown = extent * tier * (0.83 + 0.43 * bough) - x;
    } else {
        float height_delta = (p.y - 0.62) * 2.74;
        crown = sqrt(max(0.0, 1.0 - height_delta * height_delta)) * (0.43 + 0.72 * bough) - x;
        crown -= 0.33 * smoothstep(0.30, 0.65, 1.0 - bough);
    }
    float stem = mix(0.039, 0.005, p.y) - x;
    float canopy = crown + (twigs - 0.56) * 0.19;
    if (max(canopy, stem) < 0.0) { discard; }
    vec3 shade = conifer ? vec3(0.019, 0.041, 0.019) : vec3(0.026, 0.061, 0.016);
    vec3 leaves = shade * (0.49 + bough * 0.67 + twigs * 0.49 + p.y * 0.31);
    ALBEDO = mix(vec3(0.058, 0.042, 0.024), leaves, step(0.0, canopy));
    AO = mix(0.66, 1.0, bough);
    AO_LIGHT_AFFECT = 0.35;
    ROUGHNESS = 1.0;
}
