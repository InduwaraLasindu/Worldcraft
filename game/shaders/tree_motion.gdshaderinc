// Keep the trunks, branches and leaf insertion points attached in the wind.
vec3 move_tree(vec3 p, mat4 model, vec4 sample_data, float height, vec2 wind, float clock, float flex) {
    float level = clamp(p.y / height, 0.0, 1.08);
    float clear = sin(3.14159 * min(level, 1.0)) * smoothstep(0.05, 0.37, level);
    p.y += (sample_data.a - 0.5) * height * 0.20 * clear;
    p.xz *= 1.0 + (sample_data.a - 0.5) * 0.15 * smoothstep(0.38, 0.84, level);
    vec2 heading = vec2(dot(normalize(model[0].xz), wind), dot(normalize(model[2].xz), wind));
    float phase = sample_data.r * 11.2 + sample_data.g * 4.8;
    float gust = 0.28 + 0.13 * sin(clock * 1.57 + phase) + 0.062 * sin(clock * 2.96 + phase * 2.73);
    p.xz += heading * level * level * gust * flex;
    return p;
}
