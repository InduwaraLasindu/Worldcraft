shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx;

uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap_anisotropic;
uniform vec3 seed_shift;
uniform vec3 world_shift;
uniform float wetness = 0.0;
uniform float snowfall = 0.0;
uniform float grit = 0.0;
uniform int surface_type = 0;
uniform float relief = 1.0;
varying vec3 site;

void vertex() {
    site = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz + world_shift;
}

float grain(vec2 p) {
    return texture(ground_field, p).r;
}

void fragment() {
    vec3 p = site + seed_shift;
    vec3 face = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
    float slope = smoothstep(0.20, 0.71, 1.0 - max(0.0, face.y));
    float exposure = grain(p.xz * 0.012);
    float layer = grain(p.xz * 0.065 + vec2(p.y * 0.027, -p.y * 0.013));
    // Use world-locked scale changes rather than switching texture samples at
    // chunk boundaries. The browser-sized repeat never enters the geometry.
    vec2 uv = mat2(vec2(0.76, 0.65), vec2(-0.65, 0.76)) * p.xz * 0.38
        + vec2(exposure, -layer) * 0.77;
    float fine = grain(uv);
    float chips = grain(p.xz * 1.28 + vec2(layer * 0.27, exposure * 0.19));
    float ridge = 1.0 - abs(2.0 * layer - 1.0);
    vec3 base = COLOR.rgb;
    float sand = smoothstep(0.12, 0.29, base.r - base.b)
        * (1.0 - smoothstep(0.8, 1.08, base.g / max(base.r, 0.01)));
    float old_snow = smoothstep(0.56, 0.80, min(base.r, base.b));
    float ash = 1.0 - smoothstep(0.04, 0.21, max(max(base.r, base.g), base.b));
    float moss = smoothstep(0.69, 1.02, base.g / max(base.r, 0.01))
        * (1.0 - slope) * smoothstep(0.36, 0.67, layer);
    vec3 soil = base * (0.47 + 0.40 * fine + 0.23 * exposure);
    vec3 roots = vec3(0.077, 0.058, 0.039) * (0.64 + 0.77 * fine);
    soil = mix(soil, roots, moss * smoothstep(0.39, 0.76, exposure) * 0.68);
    float ripple = sin(dot(p.xz, vec2(15.0, -3.3)) + 5.1 * layer + 4.0 * exposure);
    float resolved = 1.0 - smoothstep(0.09, 0.30, length(fwidth(p.xz)));
    soil = mix(soil, vec3(0.38, 0.24, 0.127) * (0.77 + 0.39 * exposure + 0.17 * chips
        + ripple * 0.050 * resolved), sand * 0.86);
    vec3 mineral = mix(vec3(0.098, 0.089, 0.075), vec3(0.28, 0.29, 0.27), fine);
    mineral *= 0.74 + 0.43 * ridge + 0.14 * sin(p.y * 2.6 + 6.0 * exposure);
    mineral = mix(mineral, vec3(0.067, 0.052, 0.045) * (0.62 + chips), ash*0.82);
    float rock = max(slope, float(surface_type == 1));
    vec3 pigment = mix(soil, mineral, rock);
    float snow = max(old_snow, snowfall) * smoothstep(0.40, 0.91, face.y + (layer - 0.5) * 0.41);
    float wet = wetness * (1.0 - snow) * (0.35 + fine * 0.60) * (1.0 - 0.35 * slope);
    pigment = mix(pigment, pigment * vec3(0.58, 0.64, 0.67), wet);
    pigment = mix(pigment, vec3(0.44, 0.28, 0.13), grit * 0.17 * (1.0 - snow));
    ALBEDO = mix(pigment, vec3(0.74, 0.79, 0.83) * (0.76 + 0.31 * fine), snow);
    ROUGHNESS = mix(mix(0.85 - 0.13 * rock + (fine - 0.5) * 0.12, 0.34, wet), 0.88, snow);
    SPECULAR = mix(0.32, 0.57, wet);
    float micro = mix(fine + 0.32 * chips, layer + 0.68 * fine, rock) * (1.0 - snow * 0.66);
    // Reconstruct a differential bump from the same scalar as the pigments.
    vec3 sx = dFdx(VERTEX);
    vec3 sy = dFdy(VERTEX);
    vec3 x_grad = cross(sy, NORMAL);
    vec3 y_grad = cross(NORMAL, sx);
    float denominator = dot(sx, x_grad);
    if (abs(denominator) > 0.000001) {
        vec3 delta = (dFdx(micro) * x_grad + dFdy(micro) * y_grad) / denominator;
        NORMAL = normalize(NORMAL - delta * relief * 0.066);
    }
}
