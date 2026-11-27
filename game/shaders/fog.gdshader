shader_type fog;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform float density = 0.18;
uniform bool plunge;
uniform vec2 drift;

void fog() {
    vec2 source = plunge ? UVW.xz * 51.0 : WORLD_POSITION.xz;
    vec2 p = (source + seed_shift.xz) * 0.014 - drift * 0.012;
    float bank = texture(ground_field, p).r;
    float side = max(abs(UVW.x * 2.0 - 1.0), abs(UVW.z * 2.0 - 1.0));
    float edge = 1.0 - smoothstep(0.65, 1.0, side);
    float bottom = pow(1.0 - clamp(UVW.y, 0.0, 1.0), 1.6);
    DENSITY = density * edge * bottom * (0.25 + bank * 1.30);
    if (plunge) {
        vec3 local = UVW * 2.0 - 1.0;
        local.z += UVW.y * 0.62 - 0.22;
        float plume = 1.0 - smoothstep(0.18, 1.0, length(local * vec3(0.96, 1.55, 1.31)));
        DENSITY = density * plume * (0.32 + 1.09 * bank);
    }
    ALBEDO = vec3(0.79, 0.86, 0.94);
}
