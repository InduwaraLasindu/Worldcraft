shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
#include "res://shaders/tree_motion.gdshaderinc"

uniform sampler2D foliage_atlas : repeat_disable, filter_linear_mipmap_anisotropic;
uniform vec3 pigment : source_color;
uniform vec2 wind;
uniform float clock;
uniform float wetness;
uniform float snowfall;
uniform float tree_height = 12.5;
uniform float flex = 1.0;
uniform int family;
varying vec4 tree_data;
varying float shade;

void vertex() {
    tree_data = INSTANCE_CUSTOM;
    shade = COLOR.r;
    VERTEX = move_tree(VERTEX, MODEL_MATRIX, tree_data, tree_height, wind, clock, flex);
    vec2 breeze = vec2(sin(clock * 4.2 + VERTEX.y * 8.4 + tree_data.r * 8.6),
                       cos(clock * 3.6 - VERTEX.x * 4.1 + tree_data.g * 9.0));
    VERTEX.xz += breeze * min(length(wind), 1.8) * 0.015 * clamp(length(VERTEX.xz), 0.0, 3.0);
}

void fragment() {
    bool palm = family == 2;
    bool needle = family == 1;
    vec4 detail = palm ? vec4(0.58 + 0.09 * cos(UV.x * 6.0),
        0.72 * exp(-abs(UV.x - 0.5) * 26.0), 1.0, 1.0) : texture(foliage_atlas, UV);
    if (detail.a < (needle ? 0.28 : 0.36)) { discard; }
    float cell = smoothstep(0.24, 0.82, detail.r);
    vec3 leaf = pigment * (0.62 + 0.76 * cell + 0.12 * detail.g) * (0.54 + 0.49 * shade);
    leaf *= mix(vec3(0.83, 0.94, 0.99), vec3(1.16, 1.04, 0.83), tree_data.g);
    leaf *= 0.82 + 0.28 * tree_data.b;
    leaf = mix(vec3(0.067, 0.040, 0.026), leaf, smoothstep(0.22, 0.81, detail.b));
    vec3 world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    float snow = snowfall * smoothstep(0.16, 0.95, abs(world_normal.y) * 0.73 + cell * 0.36) * (needle ? 0.86 : 0.64);
    ALBEDO = mix(leaf * (1.0 - 0.29 * wetness), vec3(0.69, 0.76, 0.81), snow);
    ROUGHNESS = mix(0.68 - float(palm) * 0.22 + (1.0 - cell) * 0.18, 0.37, wetness);
    SPECULAR = 0.32 + wetness * 0.16;
    AO = 0.66 + shade * 0.34;
    SSS_STRENGTH = mix(0.20, 0.34, detail.b) * (1.0 - snow);
    SSS_TRANSMITTANCE_COLOR = vec4(0.41, 0.53, 0.12, 0.78);
    SSS_TRANSMITTANCE_DEPTH = needle ? 0.018 : 0.028;
    SSS_TRANSMITTANCE_BOOST = 0.19;
    BACKLIGHT = leaf * (0.18 + 0.16 * detail.b) * (1.0 - snow);
}
