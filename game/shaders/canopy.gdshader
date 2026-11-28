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
    vec2 texel_footprint = fwidth(UV) * vec2(1024.0, 512.0);
    float minified = smoothstep(1.0, 8.0, max(texel_footprint.x, texel_footprint.y));
    // Preserve fine leaves and needles as their coverage averages into the mip chain.
    if (detail.a < (needle ? 0.28 : 0.36) - minified * 0.10) { discard; }

    float foliage = smoothstep(0.29, 0.76, detail.b);
    float pigment_density = smoothstep(0.25, 0.85, detail.r);
    float vein = detail.g * (1.0 - 0.72 * minified);
    vec3 leaf = pigment * (0.58 + 0.81 * pigment_density + 0.18 * vein);
    leaf *= mix(vec3(0.86, 0.97, 1.03), vec3(1.19, 1.03, 0.78), tree_data.g);
    leaf *= (0.79 + 0.31 * tree_data.b) * (0.50 + 0.57 * shade);
    leaf = mix(vec3(0.071, 0.050, 0.028), leaf, foliage);
    vec3 world_normal = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    float snow = snowfall * smoothstep(0.16, 0.95, abs(world_normal.y) * 0.73 + pigment_density * 0.36)
        * (needle ? 0.86 : 0.64);
    ALBEDO = mix(leaf * (1.0 - 0.29 * wetness), vec3(0.69, 0.76, 0.81), snow);
    float cuticle_roughness = mix(0.73, 0.49, pigment_density) - (palm ? 0.07 : 0.0) + vein * 0.10;
    ROUGHNESS = mix(mix(0.86, cuticle_roughness, foliage), 0.31, wetness * foliage);
    SPECULAR = mix(0.22, 0.38 + wetness * 0.14, foliage);
    AO = mix(0.48, 1.0, shade);
    AO_LIGHT_AFFECT = 0.18;

    float thickness = clamp(0.45 * pigment_density + 0.35 * vein + (needle ? 0.29 : 0.12), 0.0, 1.0);
    float transmission = foliage * (1.0 - snow) * mix(0.43, 0.18, thickness);
    SSS_STRENGTH = transmission * 0.53;
    SSS_TRANSMITTANCE_COLOR = vec4(0.59, 0.66, 0.23, 0.82);
    SSS_TRANSMITTANCE_DEPTH = needle ? 0.015 : 0.025;
    SSS_TRANSMITTANCE_BOOST = 0.12;
    // Forward+ multiplies the shadowed direct response by the albedo.
    BACKLIGHT = vec3(1.04, 1.0, 0.53) * transmission;
}
