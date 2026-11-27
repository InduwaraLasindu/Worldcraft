shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 world_shift;
uniform vec3 seed_shift;
uniform float clock;
uniform vec2 wind;
uniform bool inland;
varying vec3 site;
varying vec3 channel;

void vertex() {
    site = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz + world_shift;
    channel = VERTEX;
}

void fragment() {
    vec3 n = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    vec2 p = site.xz + seed_shift.xz;
    float reach = texture(ground_field, p * 0.023 + wind * clock * 0.001).r;
    vec2 advect = inland ? vec2(0.0, clock * 0.075) : wind * clock * 0.004;
    float filaments = texture(ground_field, p * vec2(0.19, inland ? 0.056 : 0.16)
        + vec2(reach * 0.48, -reach * 0.36) + advect).r;

    float footprint = length(fwidth(p));
    float close_detail = 1.0 - smoothstep(0.15, 0.81, footprint);
    float swell = sin(dot(p, vec2(0.43, 0.29)) - clock * 1.29 + reach * 5.4);
    float chop = sin(dot(p, vec2(-1.78, 2.43)) + clock * 2.7 + filaments * 6.2);
    float capillary = sin(dot(p, vec2(5.11, -4.02)) - clock * 5.7 + filaments * 12.6) * close_detail;
    vec3 bump = vec3(swell * 0.047 + chop * 0.046 + capillary * 0.026, 0.0,
        swell * 0.018 - chop * 0.071 + capillary * 0.019);
    bump *= (0.45 + reach * 0.65 + min(length(wind), 2.0) * 0.29) * (inland ? 0.65 : 1.0);
    float foam = 0.0;
    float beach = 0.0;
    float sheet = 0.0;
    if (inland) {
        float bank = abs(UV.x * 2.0 - 1.0);
        beach = smoothstep(0.51, 0.98, bank + (reach - 0.5) * 0.13);
        float ribbons = texture(ground_field, vec2(channel.x * 0.29 + seed_shift.x * 0.012,
            channel.y * 0.070 + clock * 0.25 + reach * 0.39)).r;
        sheet = smoothstep(0.12, 0.61, 1.0 - abs(n.y));
        float whitewater = sheet * (0.53 + 0.45 * smoothstep(0.21, 0.74, ribbons));
        float bubbles = COLOR.r * smoothstep(0.32, 0.67, filaments + 0.20 * reach) * (0.52 + 0.48 * ribbons);
        float shoreline = smoothstep(0.81, 1.0, bank + (filaments - 0.52) * 0.11)
            * smoothstep(0.39, 0.76, filaments) * 0.63;
        foam = max(whitewater, max(bubbles, shoreline));
        bump.x += sheet * (ribbons - 0.5) * 0.39;
    }
    vec3 deep = inland ? vec3(0.022, 0.059, 0.051) : vec3(0.012, 0.048, 0.061);
    vec3 shallow = vec3(0.120, 0.161, 0.103);
    vec3 water = mix(deep, shallow, beach) * (0.81 + 0.31 * filaments);
    ALBEDO = mix(water, vec3(0.54, 0.63, 0.64) * (0.79 + 0.25 * filaments), foam);
    NORMAL = normalize((VIEW_MATRIX * vec4(normalize(n + bump), 0.0)).xyz);
    ROUGHNESS = mix(0.088 + filaments * 0.075 + length(wind) * 0.02, 0.72, foam);
    SPECULAR = mix(0.48, 0.31, foam);
}
