extends Node3D
## Seeded regional weather with bounded GPU particles and material deposition.

const LABELS := ["Automatic", "Clear", "Rain", "Thunderstorm", "Snow", "Sandstorm", "Hurricane"]
var selected := 0
var condition := 1
var air := Vector3.ZERO
var wet := 0.0
var snow := 0.0
var dust := 0.0
var wind := Vector2.ZERO
var drift := Vector2.ZERO
var flash := 0.0
var elapsed := 0.0
var rng := RandomNumberGenerator.new()
var weather_front := FastNoiseLite.new()
var collectors: Array[GPUParticles3D] = []
var strike_at := 0.0
var thunder_delay := 0.0
var thunder: AudioStreamPlayer


func _ready() -> void:
	for i in 3:
		var node := GPUParticles3D.new()
		node.emitting = false
		node.amount = [360, 250, 220][i]
		node.lifetime = [1.15, 3.1, 2.1][i]
		node.visibility_aabb = AABB(Vector3(-34.0, -19.0, -34.0), Vector3(68.0, 40.0, 68.0))
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var behavior := ParticleProcessMaterial.new()
		behavior.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
		behavior.emission_box_extents = Vector3(13.0, 2.5, 13.0)
		behavior.direction = Vector3(0.06, -0.98, 0.07) if i != 2 else Vector3(0.8, -0.3, 0.4)
		behavior.spread = [7.0, 25.0, 31.0][i]
		behavior.initial_velocity_min = [8.5, 1.7, 4.0][i]
		behavior.initial_velocity_max = [13.0, 3.3, 9.5][i]
		behavior.gravity = Vector3(0.0, -6.0 if i == 0 else -0.55, 0.0)
		behavior.scale_min = 0.56
		behavior.scale_max = 1.31
		node.process_material = behavior
		var particle := QuadMesh.new()
		particle.size = [Vector2(0.020, 0.44), Vector2(0.047, 0.047), Vector2(0.11, 0.055)][i]
		var material := StandardMaterial3D.new()
		material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		material.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
		material.albedo_color = [Color(0.66, 0.79, 0.92, 0.34), Color(0.96, 0.97, 1.0, 0.65), Color(0.63, 0.41, 0.18, 0.19)][i]
		var radial := GradientTexture2D.new()
		radial.width = 32
		radial.height = 32
		radial.gradient = Gradient.new()
		radial.gradient.colors = PackedColorArray([Color.WHITE, Color(1, 1, 1, 0)])
		radial.fill = GradientTexture2D.FILL_RADIAL
		radial.fill_from = Vector2(0.5, 0.5)
		radial.fill_to = Vector2(0.99, 0.5)
		material.albedo_texture = radial
		particle.material = material
		node.draw_pass_1 = particle
		add_child(node)
		collectors.append(node)
	thunder = AudioStreamPlayer.new()
	thunder.stream = thunder_clip()
	thunder.volume_db = -11.0
	add_child(thunder)


func restart(value: int) -> void:
	rng.seed = value ^ 15038861
	weather_front.seed = int(value & 0x7fffffff)
	weather_front.frequency = 0.00036
	elapsed = 0.0
	flash = 0.0
	air = Vector3.ZERO
	wet = 0.0
	snow = 0.0
	dust = 0.0
	drift = Vector2.ZERO
	strike_at = 2.8
	thunder_delay = 0.0


func choose(climate: Dictionary, site: Vector3, sea_level: float) -> int:
	var front := weather_front.get_noise_3d(site.x, elapsed * 3.1, site.z) + 0.26
	var moisture: float = climate.get("humidity", 0.56)
	var temperature: float = climate.get("temperature", 0.46)
	var kind: int = climate.get("kind", 2)
	if temperature > 0.70 && moisture > 0.69 && site.y < sea_level + 59.0 && front > 0.67:
		return 6
	if kind == 6 || (kind == 10 && moisture < 0.36):
		return 5 if front > 0.35 else 1
	if temperature < 0.21:
		return 4 if front > 0.13 else 1
	if moisture > 0.61 && front > 0.51:
		return 3
	return 2 if moisture > 0.43 && front > 0.14 else 1


func tick(dt: float, eye: Vector3, climate: Dictionary, site: Vector3, sea_level: float, quality: int) -> void:
	elapsed += dt
	condition = selected if selected > 0 else choose(climate, site, sea_level)
	var target: Vector3 = [
		Vector3.ZERO, Vector3.ZERO, Vector3(0.52, 0, 0), Vector3(0.83, 0, 0),
		Vector3(0, 0.77, 0), Vector3(0, 0, 0.81), Vector3(1.0, 0, 0),
	][condition]
	air = air.lerp(target, 1.0 - exp(-dt * 0.92))
	wet = lerpf(wet, air.x, 1.0 - exp(-dt * (0.24 if air.x > wet else 0.037)))
	snow = lerpf(snow, air.y, 1.0 - exp(-dt * (0.15 if air.y > snow else 0.027)))
	dust = lerpf(dust, air.z, 1.0 - exp(-dt * (0.19 if air.z > dust else 0.051)))
	var heading := 0.25 + noise_seed_phase() + sin(elapsed * 0.09) * target.length() * 0.49
	var intensity := 0.38 + air.length() * (1.76 if condition == 6 else 0.94)
	wind = Vector2.from_angle(heading) * intensity * (1.0 + sin(elapsed * 0.87) * 0.22)
	drift += wind * dt
	flash *= exp(-dt * 15.0)
	if condition in [3, 6] && elapsed > strike_at:
		strike_at = elapsed + rng.randf_range(5.5, 15.0)
		thunder_delay = rng.randf_range(0.52, 1.78)
		flash = 1.0
	if thunder_delay > 0.0:
		thunder_delay -= dt
		if thunder_delay <= 0.0:
			thunder.play()
	for i in collectors.size():
		var node := collectors[i]
		node.position = eye + Vector3(wind.x * -2.0, 5.0, wind.y * -2.0)
		var strength := air[i]
		node.emitting = strength > 0.025 && site.y >= sea_level
		node.amount_ratio = strength * (0.37 if quality == 0 else 0.72 if quality == 1 else 1.0)
		var behavior := node.process_material as ParticleProcessMaterial
		behavior.direction = Vector3(wind.x * 0.31, -0.85 if i != 2 else -0.12, wind.y * 0.31).normalized()


func noise_seed_phase() -> float:
	return float(weather_front.seed % 180) * TAU / 180.0


func thunder_clip() -> AudioStreamWAV:
	var random := RandomNumberGenerator.new()
	random.seed = 4477152
	var bytes := PackedByteArray()
	var rate := 22050
	bytes.resize(rate * 2 * 2)
	var low := 0.0
	var roll := 0.0
	for i in rate * 2:
		var t := float(i) / rate
		low += 0.09 * (random.randf_range(-1.0, 1.0) - low)
		roll += 0.021 * (low - roll)
		var sound := (low * (0.42 + 0.44 * exp(-t * 12.0)) + roll * 4.1 * (0.8 + 0.2 * cos(t * 14.9)))
		bytes.encode_s16(i * 2, int(sound * pow(1.0 - t * 0.5, 2.0) * minf(1.0, t * 30.0) * 32000))
	var clip := AudioStreamWAV.new()
	clip.format = AudioStreamWAV.FORMAT_16_BITS
	clip.mix_rate = rate
	clip.data = bytes
	return clip
