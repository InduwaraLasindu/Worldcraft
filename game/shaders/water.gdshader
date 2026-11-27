shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 world_shift;
uniform vec3 seed_shift;
uniform float clock;
uniform vec2 wind;
uniform bool inland;
varying vec3 site;

void vertex() {
    site = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz + world_shift;
}

void fragment() {
    vec3 n = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    vec2 p = site.xz + seed_shift.xz;
    float field = texture(ground_field, p * 0.026 + wind * clock * 0.001).r;
    float fine = texture(ground_field, p * 0.14 + vec2(field * 0.44, -field * 0.23) + wind * clock * 0.006).r;
    float swell = sin(dot(p, vec2(0.29, 0.44)) + clock * 1.23 + field * 5.2);
    float wave = sin(dot(p, vec2(-1.23, 2.37)) - clock * 1.37 + field * 4.8);
    float capillary = sin(dot(p, vec2(4.04, 5.61)) + clock * 2.88 + fine * 7.0)
        * (1.0 - smoothstep(0.14, 0.74, length(fwidth(p))));
    float stream = texture(ground_field, vec2(site.x * 0.28, site.y * 0.062 + clock * 0.12)).r;
    vec3 bump = vec3(swell * 0.045 + wave * 0.072 + capillary * 0.022, 0.0, swell * 0.037 - wave * 0.062 + capillary * 0.038)
        * (0.68 + min(length(wind), 2.0) * 0.29);
    float waterfall = inland ? smoothstep(0.01, 0.93, -UV.y) : 0.0;
    float shallow = inland ? pow(abs(UV.x * 2.0 - 1.0), 5.0) : 0.0;
    float foam = max(waterfall * smoothstep(0.29, 0.75, stream), shallow * smoothstep(0.56, 0.78, fine) * 0.47);
    vec3 water = mix(vec3(0.025, 0.086, 0.102), vec3(0.062, 0.171, 0.136), shallow);
    ALBEDO = mix(water * (0.79 + 0.40 * fine), vec3(0.57, 0.70, 0.71), foam);
    NORMAL = normalize((VIEW_MATRIX * vec4(normalize(n + bump * (1.0 - waterfall * 0.62)), 0.0)).xyz);
    ROUGHNESS = mix(0.092 + fine * 0.055 + length(wind) * 0.018, 0.58, foam);
    SPECULAR = 0.53;
}
