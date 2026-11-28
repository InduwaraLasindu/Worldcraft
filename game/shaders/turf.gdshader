shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;

uniform vec2 wind;
uniform float clock;
uniform float wetness;
uniform float snowfall;
uniform float reach = 100.0;

varying vec4 plant;
varying float blade;
varying float tint;
varying float visibility;

void vertex() {
    plant = INSTANCE_CUSTOM;
    blade = UV.y;
    tint = COLOR.r;
    vec3 root = MODEL_MATRIX[3].xyz;
    float distance_to_camera = distance(root, CAMERA_POSITION_WORLD);
    visibility = 1.0 - smoothstep(reach * 0.73, reach, distance_to_camera);
    float gust = sin(clock * 2.3 + dot(root.xz, vec2(0.19, 0.23))) * 0.25
        + sin(clock * 4.6 + plant.r * 9.0) * 0.17;
    vec2 local_wind = vec2(dot(normalize(MODEL_MATRIX[0].xz), wind), dot(normalize(MODEL_MATRIX[2].xz), wind));
    VERTEX.xz += local_wind * blade * blade * (0.086 + gust * 0.10);
}

void fragment() {
    if (visibility < plant.g * 0.89 + 0.10) { discard; }
    vec3 base = mix(vec3(0.054, 0.091, 0.017), vec3(0.145, 0.111, 0.031), plant.a);
    vec3 tip = mix(vec3(0.244, 0.305, 0.058), vec3(0.393, 0.286, 0.091), plant.a);
    vec3 grass = mix(base, tip, smoothstep(0.03, 0.84, blade));
    float seam = exp(-abs(UV.x - 0.5) * 25.0);
    grass *= (0.64 + 0.49 * tint + 0.07 * seam) * (1.0 - wetness * 0.23);
    ALBEDO = mix(grass, vec3(0.69, 0.75, 0.78), snowfall * blade * 0.78);
    ROUGHNESS = mix(0.86, 0.52, wetness);
    SPECULAR = mix(0.29, 0.42, wetness);
    AO = mix(0.53, 1.0, blade);
    BACKLIGHT = grass * (0.26 + 0.31 * blade);
}
