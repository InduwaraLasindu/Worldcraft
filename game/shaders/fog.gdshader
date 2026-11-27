shader_type fog;
uniform sampler2D ground_field : repeat_enable, filter_linear_mipmap;
uniform vec3 seed_shift;
uniform float density = 0.18;

void fog() {
    vec2 p = (WORLD_POSITION.xz + seed_shift.xz) * 0.014;
    float bank = texture(ground_field, p).r;
    float side = max(abs(UVW.x * 2.0 - 1.0), abs(UVW.z * 2.0 - 1.0));
    float edge = 1.0 - smoothstep(0.65, 1.0, side);
    float bottom = pow(1.0 - clamp(UVW.y, 0.0, 1.0), 1.6);
    DENSITY = density * edge * bottom * (0.25 + bank * 1.30);
    ALBEDO = vec3(0.79, 0.86, 0.94);
}
