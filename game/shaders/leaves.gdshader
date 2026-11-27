shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 seed_shift;
uniform vec2 wind;
uniform float clock;
uniform float wetness;
uniform float snowfall;
uniform float species;
uniform bool blade;
uniform vec3 pigment : source_color = vec3(0.19, 0.31, 0.09);
varying vec4 variation;
varying float edge_shade;

void vertex() {
    variation = INSTANCE_CUSTOM;
    float phase = dot(MODEL_MATRIX[3].xz, vec2(0.062, 0.043)) + variation.g * 7.0;
    float gust = sin(clock * 1.9 + phase) * 0.15 + sin(clock * 3.9 + phase * 2.1) * 0.052;
    float bend = blade ? clamp(VERTEX.y * 2.0, 0.0, 1.0) : clamp(VERTEX.y / 12.0, 0.0, 1.0);
    VERTEX.xz += wind * bend * bend * (0.15 + gust);
    edge_shade = COLOR.r;
}

void fragment() {
    vec2 uv = UV;
    float needle = step(0.5, species) * (1.0 - step(1.5, species));
    float fern = step(1.5, species);
    float rows = mix(6.0 + variation.g * 2.0, 16.0 + variation.g * 6.0, needle);
    float y = clamp(uv.y, 0.0, 1.0);
    float row = floor(y * rows);
    float row_salt = fract(sin(row * 83.21 + variation.r * 15.77) * 1831.531);
    float x = abs(uv.x - 0.5);
    float stagger = mix(-0.23, 0.21, step(0.5, uv.x)) * (1.0 - needle * 0.62);
    float leaflet = fract(y * rows + x * mix(-1.38, -1.89, needle) + stagger) - 0.5;
    float reach = sin(pow(y, 0.69 + 0.15 * fern) * 3.14159) * (0.24 + row_salt * 0.23);
    float segment = clamp(x / max(0.04, reach), 0.0, 1.0);
    float oval = segment * 2.0 - 1.0;
    float leaf_margin = mix(sqrt(max(0.0, 1.0 - oval * oval)), 1.0 - segment * 0.89, needle);
    float within = 1.0 - smoothstep(0.13, 0.25, abs(leaflet) / max(0.06, leaf_margin));
    float alpha = max(1.0 - smoothstep(0.009, 0.019, x), within * (1.0 - smoothstep(reach * 0.86, reach + 0.001, x)));
    if (!blade) {
        ALPHA = alpha;
        ALPHA_SCISSOR_THRESHOLD = 0.43;
    } else {
        ALPHA = 1.0;
        ALPHA_SCISSOR_THRESHOLD = 0.43;
    }
    float field = texture(ground_field, uv * 0.38 + vec2(variation.r + seed_shift.x * 0.017, variation.g)).r;
    float vessel = 1.0 - smoothstep(0.0, 0.022, abs(leaflet));
    float thickness = clamp((0.70 + field * 0.32) * (0.68 + 0.32 * edge_shade), 0.0, 1.0);
    vec3 leaf = pigment * (0.61 + 0.55 * field + 0.085 * vessel) * (0.54 + 0.53 * edge_shade);
    leaf *= mix(vec3(0.83, 0.94, 1.03), vec3(1.16, 1.02, 0.80), row_salt * (1.0 - 0.4 * needle));
    leaf *= 0.72 + 0.53 * variation.b;
    leaf = mix(leaf, leaf * vec3(0.66, 0.75, 0.81), wetness);
    leaf = mix(leaf, vec3(0.59, 0.68, 0.71), snowfall * thickness * (1.0 - 0.28 * needle));
    ALBEDO = leaf;
    AO = 0.74 + 0.26 * edge_shade;
    ROUGHNESS = mix(0.77, 0.40, wetness);
    SPECULAR = 0.37 + wetness * 0.13;
    // Thin-leaf transmission uses the engine's directional light, masks and
    // shadow cascades rather than adding green emissive light.
    SSS_STRENGTH = 0.22 + 0.18 * thickness;
    SSS_TRANSMITTANCE_COLOR = vec4(0.30 + 0.14 * fern, 0.53, 0.11, 0.85);
    SSS_TRANSMITTANCE_DEPTH = mix(0.035, 0.018, needle);
    SSS_TRANSMITTANCE_BOOST = 0.16;
    BACKLIGHT = leaf * (0.25 + 0.16 * (1.0 - thickness));
}
