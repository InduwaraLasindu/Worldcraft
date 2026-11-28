shader_type spatial;
render_mode cull_disabled, depth_draw_never, diffuse_burley, specular_schlick_ggx;

uniform sampler2D scene_color : hint_screen_texture, filter_linear_mipmap;
uniform sampler2D scene_depth : hint_depth_texture, filter_nearest;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform vec3 world_shift;
uniform vec2 wind;
uniform float clock;
uniform int reflection_steps = 11;
uniform vec3 horizon_light = vec3(0.48, 0.66, 0.87);
uniform float lake = 0.0;

varying vec3 site;

void vertex() {
    site = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz + world_shift;
}

float eye_depth(vec2 uv, mat4 inverse_proj) {
    float depth = textureLod(scene_depth, uv, 0.0).r;
    #if CURRENT_RENDERER == RENDERER_COMPATIBILITY
    vec3 ndc = vec3(uv, depth) * 2.0 - 1.0;
    #else
    vec3 ndc = vec3(uv * 2.0 - 1.0, depth);
    #endif
    vec4 p = inverse_proj * vec4(ndc, 1.0);
    return -p.z / max(0.000001, p.w);
}

vec3 reflection(vec3 start, vec3 direction, mat4 projection, mat4 inverse_proj, float rough) {
    vec3 hit = vec3(0.0);
    float previous = 0.0;
    float reach = mix(154.0, 670.0, lake);
    for (int i = 1; i <= 20; i++) {
        if (i > reflection_steps) { break; }
        float phase = float(i) / float(reflection_steps);
        float distance_along = 0.45 + reach * phase * phase;
        vec3 p = start + direction * distance_along;
        vec4 projected = projection * vec4(p, 1.0);
        if (projected.w <= 0.01) { break; }
        vec2 uv = projected.xy / projected.w * 0.5 + 0.5;
        if (min(uv.x, uv.y) <= 0.005 || max(uv.x, uv.y) >= 0.995) { break; }
        float crossing = -p.z - eye_depth(uv, inverse_proj);
        if (crossing > 0.0 && previous <= 0.0 && crossing < 2.0 + distance_along * 0.23) {
            float earlier = float(i - 1) / float(reflection_steps);
            float near = 0.45 + reach * earlier * earlier;
            float far = distance_along;
            for (int j = 0; j < 3; j++) {
                float middle = (near + far) * 0.5;
                vec3 probe = start + direction * middle;
                vec4 clip = projection * vec4(probe, 1.0);
                vec2 point = clip.xy / clip.w * 0.5 + 0.5;
                if (-probe.z > eye_depth(point, inverse_proj)) {
                    far = middle;
                    uv = point;
                } else {
                    near = middle;
                }
            }
            hit.xy = uv;
            vec2 edge = min(uv, vec2(1.0) - uv);
            hit.z = smoothstep(0.008, 0.13, min(edge.x, edge.y))
                * (1.0 - 0.29 * phase) * (1.0 - rough);
            break;
        }
        previous = crossing;
    }
    return hit;
}

void fragment() {
    vec2 p = site.xz + seed_shift.xz;
    vec2 v = wind * clock;
    float drift = texture(ground_field, p * 0.012 + v * 0.0009).r;
    float facets = texture(ground_field, vec2(p.x - p.y * 0.49, p.y + p.x * 0.31) * 0.109 + vec2(-drift, drift) * 0.39 - v * 0.012).r;
    float detail = texture(ground_field, p.yx * 0.37 + vec2(facets * 0.27, drift * 0.16) + v.yx * 0.018).r;
    float ripple = sin(dot(p, vec2(1.07, -1.53)) + drift * 15.5 + facets * 5.7 + clock * 1.43);
    float swell = sin(dot(p, vec2(0.33, 0.53)) + drift * 8.5 + clock * 0.52);
    float fine = 1.0 - smoothstep(0.10, 0.55, length(fwidth(p)));
    vec2 slope = vec2((facets - 0.5) * 0.19 + ripple * 0.025, (detail - 0.5) * 0.11 * fine + swell * 0.052);
    slope *= (0.65 + min(length(wind), 2.2) * 0.39) * mix(1.0, 0.37, lake);
    vec3 world_normal = normalize(vec3(slope.x, 1.0, slope.y));
    vec3 n = normalize((VIEW_MATRIX * vec4(world_normal, 0.0)).xyz);
    vec3 seen = normalize(VIEW);
    float water_z = -VERTEX.z;
    float depth_diff = max(0.0, eye_depth(SCREEN_UV, INV_PROJECTION_MATRIX) - water_z);
    vec2 uv = clamp(SCREEN_UV + n.xy * min(0.042, depth_diff * 0.0035), vec2(0.001), vec2(0.999));
    float bent_depth = eye_depth(uv, INV_PROJECTION_MATRIX) - water_z;
    if (bent_depth < 0.03) { uv = SCREEN_UV; bent_depth = depth_diff; }
    vec3 absorption = exp(-max(0.0, bent_depth) * mix(vec3(0.38, 0.16, 0.095), vec3(0.33, 0.109, 0.079), lake));
    vec3 bottom = textureLod(scene_color, uv, min(2.0, bent_depth * 0.066)).rgb;
    vec3 water = mix(vec3(0.019, 0.073, 0.079), vec3(0.023, 0.074, 0.068), lake);
    vec3 transmitted = bottom * absorption + water * (1.0 - absorption);
    float fresnel = 0.021 + 0.979 * pow(1.0 - max(0.0, dot(n, seen)), 5.0);
    vec3 bounced = reflect(-seen, n);
    vec3 world_bounce = (INV_VIEW_MATRIX * vec4(bounced, 0.0)).xyz;
    vec3 sky = horizon_light * mix(vec3(0.34, 0.46, 0.71), vec3(1.0), exp(-max(0.0, world_bounce.y) * 3.7));
    sky *= mix(1.0, 0.58, lake);
    float rough = mix(0.08, 0.047, lake) + facets * 0.05 + length(wind) * 0.023;
    if (reflection_steps > 0 && fresnel > 0.075 && depth_diff > 0.10) {
        vec3 hit = reflection(VERTEX + n * 0.19, bounced, PROJECTION_MATRIX, INV_PROJECTION_MATRIX, rough);
        sky = mix(sky, textureLod(scene_color, hit.xy, 0.6 + rough * 5.0).rgb, hit.z);
    }
    float foam = (1.0 - smoothstep(0.015, 0.19, depth_diff))
        * smoothstep(0.54, 0.83, detail + ripple * 0.11) * mix(0.49, 0.17, lake);
    EMISSION = mix(transmitted, sky, fresnel) * (1.0 - foam);
    ALBEDO = (water * 0.25 + vec3(0.51, 0.57, 0.54) * foam) * (1.0 - fresnel * 0.43);
    NORMAL = n;
    ROUGHNESS = mix(rough, 0.53, foam);
    SPECULAR = mix(0.43, 0.21, foam);
    ALPHA = 1.0;
}
