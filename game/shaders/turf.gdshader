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
    vec3 tip = mix(vec3(0.181, 0.243, 0.053), vec3(0.315, 0.254, 0.104), plant.a);
    vec3 grass = mix(base, tip, smoothstep(0.03, 0.84, blade));
    float minified = smoothstep(0.35, 1.15, fwidth(UV.x));
    float rib = (1.0 - smoothstep(0.005, 0.15, abs(UV.x - 0.5))) * (1.0 - minified);
    grass *= (0.62 + 0.54 * tint + 0.09 * rib) * (1.0 - wetness * 0.23);
    float snow = snowfall * blade * 0.78;
    ALBEDO = mix(grass, vec3(0.69, 0.75, 0.78), snow);
    ROUGHNESS = mix(0.83 - 0.20 * blade + 0.07 * rib, 0.40, wetness) + 0.07 * minified;
    SPECULAR = mix(0.28 + 0.10 * blade, 0.47, wetness) * (1.0 - 0.28 * minified);
    AO = mix(0.53, 1.0, blade);
    AO_LIGHT_AFFECT = 0.16;
    // The blade tip transmits more than its thick base and central rib.
    BACKLIGHT = mix(vec3(0.97, 1.0, 0.48), vec3(1.08, 0.93, 0.61), plant.a)
        * (0.14 + 0.35 * blade) * (1.0 - 0.25 * rib) * (1.0 - snow);
}
