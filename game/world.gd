extends Node3D

const Materials = preload("res://materials.gd")
const Nature = preload("res://nature.gd")
const Weather = preload("res://weather.gd")
const PRESETS := ["Temperate wilderness", "Alpine mountains", "Tropical highlands", "Desert and canyons",
	"Arctic wilderness", "Archipelago", "Grassland and savanna", "Volcanic islands", "Tidal estuary",
	"River valley", "Empty canvas"]
const TILE := 128.0
const SAVE := "user://worldcraft.worldcraft.json"

var native: WorldcraftNative
var materials := Materials.new()
var nature := Nature.new()
var weather := Weather.new()
var camera := Camera3D.new()
var sun := DirectionalLight3D.new()
var environment := Environment.new()
var valley_fog: FogVolume
var land := Node3D.new()
var ocean: MeshInstance3D
var river: MeshInstance3D
var plunge_spray: GPUParticles3D
var horizon: MeshInstance3D
var horizon_center := Vector2i(2147483647, 2147483647)
var horizon_pending := false
var center := Vector2i(2147483647, 2147483647)
var origin := Vector2i.ZERO
var tiles := {}
var requested := {}
var wanted := {}
var order: Array[Vector2i] = []
var local := {}
var active_world := false
var flying := false
var quality := 1
var radius := 2
var texture_size := 1024
var gi := true
var shafts := true
var clock := 9.0
var moving_sun := true
var day_minutes := 16.0
var yaw := 0.0
var pitch := -0.12
var up_speed := 0.0
var elapsed := 0.0
var climate_update := 0.0
var climate := {}
var settle_frames := 0
var bench_start := 0
var settled_ms: Array[float] = []
var next_audit_report := 0
var audit := false
var audit_image := ""
var seed_input: LineEdit
var preset_input: OptionButton
var status: Label
var help: Label
var bar: PanelContainer
var slider: HSlider
var wants_capture := false
var forward := true
var software := false
var adapter_rays := false
var auto_scale := false
var average_frame := 16.7
var next_scale := 0.0
var editing := false
var brush_kind := 0
var brush_biome := 3
var brush_radius := 24.0
var edit_panel: HBoxContainer


func _ready() -> void:
	forward = RenderingServer.get_current_rendering_method() == "forward_plus"
	var adapter := RenderingServer.get_video_adapter_name().to_lower()
	software = adapter.contains("llvmpipe") || adapter.contains("software") || adapter.contains("swiftshader")
	var device := RenderingServer.get_rendering_device()
	adapter_rays = device != null && device.has_feature(RenderingDevice.SUPPORTS_RAY_QUERY)
	if not ClassDB.class_exists("WorldcraftNative"):
		push_error("Worldcraft Rust library is missing. Build and stage the GDExtension first.")
		get_tree().quit(1)
		return
	native = ClassDB.instantiate("WorldcraftNative")
	materials.initialize(42)
	add_child(land)
	add_child(camera)
	camera.current = true
	camera.fov = 68.0
	camera.near = 0.12
	camera.far = 22000.0
	add_child(sun)
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 280.0
	sun.directional_shadow_blend_splits = true
	sun.light_angular_distance = 0.53
	sun.light_volumetric_fog_energy = 1.12
	var world_environment := WorldEnvironment.new()
	environment.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	sky.sky_material = materials.sky
	sky.radiance_size = Sky.RADIANCE_SIZE_64 if software else Sky.RADIANCE_SIZE_128
	sky.process_mode = Sky.PROCESS_MODE_INCREMENTAL
	environment.sky = sky
	environment.tonemap_mode = Environment.TONE_MAPPER_AGX
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.57, 0.69, 0.87)
	environment.ambient_light_energy = 0.31
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	environment.ssao_radius = 1.8
	environment.ssao_intensity = 1.32
	environment.ssil_radius = 3.4
	environment.ssil_intensity = 0.63
	environment.sdfgi_min_cell_size = 0.62
	environment.sdfgi_use_occlusion = true
	environment.sdfgi_bounce_feedback = 0.28
	if software:
		environment.sdfgi_cascades = 2
		RenderingServer.environment_set_sdfgi_ray_count(RenderingServer.ENV_SDFGI_RAY_COUNT_4)
	environment.volumetric_fog_density = 0.0008
	environment.volumetric_fog_length = 350.0
	environment.volumetric_fog_detail_spread = 2.1
	environment.volumetric_fog_anisotropy = 0.72
	environment.volumetric_fog_ambient_inject = 0.14
	environment.volumetric_fog_gi_inject = 0.16
	world_environment.environment = environment
	add_child(world_environment)
	valley_fog = FogVolume.new()
	valley_fog.shape = RenderingServer.FOG_VOLUME_SHAPE_BOX
	valley_fog.size = Vector3(740, 45, 740)
	valley_fog.material = materials.fog
	add_child(valley_fog)
	add_child(weather)
	_make_ui()
	_apply_quality()
	var args := OS.get_cmdline_user_args()
	audit = "--audit" in args
	if audit:
		quality = clampi(int(_arg(args, "--quality", "1")), 0, 2)
		texture_size = int(_arg(args, "--texture", "512"))
		radius = 1
		moving_sun = false
		clock = float(_arg(args, "--hour", "8.0"))
		gi = "--gi" in args
		shafts = "--clear-air" not in args
		weather.selected = clampi(int(_arg(args, "--weather", "1")), 0, 6)
		audit_image = _arg(args, "--image", "")
		bar.hide()
		help.hide()
		_apply_quality()
		_new_world(_arg(args, "--seed", "42"), int(_arg(args, "--preset", "9")))
		if "--waterfall" in args:
			var view := native.waterfall_camera()
			camera.position = Vector3(view[0] - origin.x * TILE, view[1], view[2] - origin.y * TILE)
			camera.look_at(Vector3(view[3] - origin.x * TILE, view[4], view[5] - origin.y * TILE))
			yaw = camera.rotation.y
			pitch = camera.rotation.x
			flying = true
		if "--focus-tree" in args:
			var view := native.tree_view(int(_arg(args, "--focus-tree", "0")))
			if view.is_empty():
				push_error("The audit could not find the requested tree family near spawn")
				get_tree().quit(1)
				return
			var offset := Vector3(origin.x * TILE, 0.0, origin.y * TILE)
			camera.position = Vector3(view[0], view[1], view[2]) - offset
			camera.look_at(Vector3(view[3], view[4], view[5]) - offset)
			yaw = camera.rotation.y
			pitch = camera.rotation.x
		if "--look-sun" in args:
			var angle := (clock - 6.0) * TAU / 24.0
			var direction := Vector3(cos(angle), 0.065, 0.38)
			camera.look_at(camera.position + direction)
			yaw = camera.rotation.y
			pitch = camera.rotation.x
		if "--rise" in args:
			camera.position.y += float(_arg(args, "--rise", "0"))
		if "--yaw" in args || "--pitch" in args:
			yaw = deg_to_rad(float(_arg(args, "--yaw", str(rad_to_deg(yaw)))))
			pitch = deg_to_rad(float(_arg(args, "--pitch", str(rad_to_deg(pitch)))))
			camera.rotation = Vector3(pitch, yaw, 0)
		bench_start = Time.get_ticks_msec()
	else:
		_new_world("42", 9)


func _arg(values: PackedStringArray, name: String, fallback: String) -> String:
	var index := values.find(name)
	return values[index + 1] if index >= 0 && index + 1 < values.size() else fallback


func _new_world(seed_text: String, preset: int) -> void:
	_begin(native.create_world(seed_text, preset))


func _begin(info: Dictionary) -> void:
	if info.has("error"):
		status.text = info.error
		return
	for child in land.get_children():
		land.remove_child(child)
		child.queue_free()
	tiles.clear()
	requested.clear()
	order.clear()
	wanted.clear()
	ocean = null
	river = null
	plunge_spray = null
	horizon = null
	horizon_pending = false
	center = Vector2i(2147483647, 2147483647)
	horizon_center = center
	local = info
	origin = Vector2i(floori(info.x / TILE), floori(info.z / TILE))
	camera.position = Vector3(info.x - origin.x * TILE, info.y, info.z - origin.y * TILE)
	yaw = info.yaw
	pitch = info.pitch
	camera.rotation = Vector3(pitch, yaw, 0)
	up_speed = 0.0
	climate_update = 0.0
	nature.reset(materials, native, info.seed)
	weather.restart(info.seed)
	materials.request(info.seed, texture_size)
	var sea_plane := PlaneMesh.new()
	sea_plane.size = Vector2(34000, 34000)
	ocean = MeshInstance3D.new()
	ocean.mesh = sea_plane
	ocean.material_override = materials.sea
	ocean.position.y = info.sea - 0.025
	ocean.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	ocean.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	land.add_child(ocean)
	var network := native.river()
	if not network.is_empty():
		river = _mesh(network, materials.river)
		river.position = Vector3(-origin.x * TILE, 0.0, -origin.y * TILE)
		river.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		river.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		land.add_child(river)
		_build_plunge(network.plunge, network.plunge_half_width, info.seed)
	active_world = true
	_sunlight(0.0)


func _build_plunge(pool: Vector3, half_width: float, seed_value: int) -> void:
	var point := pool - Vector3(origin.x * TILE, 0.0, origin.y * TILE)
	var mist := FogVolume.new()
	mist.shape = RenderingServer.FOG_VOLUME_SHAPE_BOX
	mist.size = Vector3(half_width * 2.45, 20.5, half_width * 2.1)
	mist.position = point + Vector3(0.0, 3.1, -4.5)
	mist.material = materials.plunge
	land.add_child(mist)
	plunge_spray = GPUParticles3D.new()
	plunge_spray.position = point + Vector3(0.0, 0.4, 0.0)
	plunge_spray.amount = 144
	plunge_spray.amount_ratio = [0.32, 0.66, 1.0][quality]
	plunge_spray.lifetime = 2.5
	plunge_spray.preprocess = 2.0
	plunge_spray.local_coords = true
	plunge_spray.use_fixed_seed = true
	plunge_spray.seed = seed_value & 0xffffffff
	plunge_spray.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	plunge_spray.visibility_aabb = AABB(Vector3(-half_width - 12.0, -3.0, -24.0), Vector3(2.0 * half_width + 24.0, 22.0, 35.0))
	var motion := ParticleProcessMaterial.new()
	motion.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	motion.emission_box_extents = Vector3(half_width * 0.84, 0.6, 2.5)
	motion.direction = Vector3(0.0, 0.66, -0.76)
	motion.spread = 43.0
	motion.initial_velocity_min = 2.8
	motion.initial_velocity_max = 7.6
	motion.gravity = Vector3(0.0, -3.5, -0.4)
	motion.scale_min = 0.31
	motion.scale_max = 1.17
	var fade := GradientTexture1D.new()
	fade.gradient = Gradient.new()
	fade.gradient.offsets = PackedFloat32Array([0.0, 0.13, 0.46, 1.0])
	fade.gradient.colors = PackedColorArray([Color(1, 1, 1, 0), Color.WHITE, Color(1, 1, 1, 0.46), Color(1, 1, 1, 0)])
	motion.color_ramp = fade
	plunge_spray.process_material = motion
	var droplets := StandardMaterial3D.new()
	droplets.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	droplets.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	droplets.vertex_color_use_as_albedo = true
	droplets.albedo_color = Color(0.88, 0.93, 0.97, 0.58)
	droplets.albedo_texture = Weather.soft_particle()
	droplets.roughness = 0.9
	var quad := QuadMesh.new()
	quad.size = Vector2(0.75, 0.46)
	quad.material = droplets
	plunge_spray.draw_pass_1 = quad
	land.add_child(plunge_spray)


func _mesh(data: Dictionary, material: Material) -> MeshInstance3D:
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, Nature.mesh_arrays(data))
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.material_override = material
	instance.gi_mode = GeometryInstance3D.GI_MODE_STATIC
	return instance


func _site() -> Vector3:
	return camera.position + Vector3(origin.x * TILE, 0, origin.y * TILE)


func _layout(focus: Vector2i) -> void:
	wanted.clear()
	order.clear()
	for z in range(-radius, radius + 1):
		for x in range(-radius, radius + 1):
			var coord := focus + Vector2i(x, z)
			var distance := maxi(absi(x), absi(z))
			var lod := 0 if distance <= 1 else 1 if distance <= 3 else 2
			var cover := (2 if quality == 0 else 0) if distance <= (1 if quality > 0 else 0) else 1
			wanted[coord] = Vector2i(lod, cover)
			order.append(coord)
	order.sort_custom(func(a: Vector2i, b: Vector2i) -> bool:
		return (a - focus).length_squared() < (b - focus).length_squared())
	for key: Vector2i in tiles.keys():
		if not wanted.has(key):
			tiles[key].node.queue_free()
			tiles.erase(key)


func _edges(tile: Vector2i, lod: int) -> int:
	var north: Vector2i = wanted.get(tile + Vector2i.UP, Vector2i(lod, 0))
	var east: Vector2i = wanted.get(tile + Vector2i.RIGHT, Vector2i(lod, 0))
	var south: Vector2i = wanted.get(tile + Vector2i.DOWN, Vector2i(lod, 0))
	var west: Vector2i = wanted.get(tile + Vector2i.LEFT, Vector2i(lod, 0))
	return north.x | (east.x << 2) | (south.x << 4) | (west.x << 6)


func _neighbors(bits: int) -> Vector4i:
	return Vector4i(bits & 3, (bits >> 2) & 3, (bits >> 4) & 3, (bits >> 6) & 3)


func _stream() -> void:
	var focus := origin + Vector2i(floori(camera.position.x / TILE), floori(camera.position.z / TILE))
	if focus != center:
		center = focus
		_layout(focus)
	var data := native.poll_chunk()
	if not data.is_empty():
		var key := Vector2i(data.x, data.z)
		if data.horizon:
			horizon_pending = false
			if horizon != null:
				horizon.queue_free()
			horizon = _mesh(data, materials.vista)
			horizon.position = Vector3((key.x - origin.x) * TILE, -0.25, (key.y - origin.y) * TILE)
			horizon.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
			horizon.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			land.add_child(horizon)
			horizon_center = key
		else:
			var finished = requested.get(key, null)
			requested.erase(key)
			if wanted.has(key) && finished != null && data.lod == wanted[key].x && finished.y == wanted[key].y && finished.z == _edges(key, data.lod):
				var tile := _mesh(data, materials.terrain)
				tile.position = Vector3((key.x - origin.x) * TILE, 0.0, (key.y - origin.y) * TILE)
				land.add_child(tile)
				var scenery := nature.attach(tile, data.nature, finished.y)
				if tiles.has(key):
					tiles[key].node.queue_free()
				tiles[key] = {"node": tile, "build": finished, "scenery": scenery}
	var issued := 0
	for key in order:
		if requested.has(key):
			continue
		var plan: Vector2i = wanted[key]
		var build := Vector3i(plan.x, plan.y, _edges(key, plan.x))
		if tiles.has(key) && tiles[key].build == build:
			continue
		if not native.request_tile(key.x, key.y, plan.x, plan.y, _neighbors(build.z)):
			break
		requested[key] = build
		issued += 1
		if issued == 2:
			break
	if requested.is_empty() && not horizon_pending && (horizon == null || horizon_center.distance_squared_to(center) > 4):
		horizon_pending = native.request_horizon(center.x, center.y, radius, 12288.0 if quality == 2 else 8192.0)


func _rebase() -> void:
	var step := Vector2i(roundi(camera.position.x / TILE), roundi(camera.position.z / TILE))
	if maxi(absi(step.x), absi(step.y)) < 5:
		return
	var delta := Vector3(step.x * TILE, 0.0, step.y * TILE)
	origin += step
	camera.position -= delta
	for child: Node3D in land.get_children():
		child.position -= delta
	ocean.position.x = camera.position.x
	ocean.position.z = camera.position.z


func _process(delta: float) -> void:
	if not active_world:
		return
	var dt := minf(delta, 0.065)
	if not audit:
		_move(dt)
		_adjust_scale(delta)
	_rebase()
	_stream()
	elapsed += dt
	if moving_sun:
		clock = fposmod(clock + dt * 0.4 / day_minutes, 24.0)
	var site := _site()
	if elapsed >= climate_update:
		climate = native.climate(site.x, site.z)
		climate_update = elapsed + 0.65
		if not audit:
			status.text = "%s  ·  %s  ·  %02d:%02d  ·  %s  ·  %d objects  ·  %d FPS" % [
				climate.get("biome", "Exploring"), Weather.LABELS[weather.condition], int(clock),
				int(fposmod(clock, 1.0) * 60.0), "%d px" % materials.active_side, _scenery_totals().x, Engine.get_frames_per_second()]
	weather.tick(dt, camera.position, climate, site, local.sea, quality)
	materials.animate(elapsed, weather.wind, weather.wet, weather.snow, weather.dust,
		Vector3(fposmod(origin.x * TILE, 64000.0), 0.0, fposmod(origin.y * TILE, 64000.0)))
	_sunlight(dt)
	if audit:
		_check_audit(delta)


func _sunlight(_dt: float) -> void:
	var a := (clock - 6.0) * TAU / 24.0
	var sun_axis := Vector3(cos(a), sin(a), 0.38).normalized()
	sun.look_at_from_position(sun_axis * 100.0, Vector3.ZERO)
	var daylight := smoothstep(-0.12, 0.26, sun_axis.y)
	var warm := 1.0 - smoothstep(0.02, 0.54, sun_axis.y)
	var overcast := clampf(weather.air.x * 0.86 + weather.air.y * 0.66 + weather.air.z * 0.38, 0.0, 1.0)
	sun.light_color = Color(1, 0.55, 0.32).lerp(Color(1.0, 0.95, 0.85), 1.0 - warm * 0.72)
	sun.light_energy = 1.25 * sqrt(maxf(0.0, sun_axis.y)) * (1.0 - overcast * 0.77) + weather.flash * 1.4
	environment.ambient_light_color = Color(0.29, 0.39, 0.58).lerp(Color(0.65, 0.75, 0.91), daylight)
	environment.ambient_light_energy = (0.071 + daylight * (0.29 + overcast * 0.11)) + weather.flash * 0.27
	environment.fog_enabled = true
	environment.fog_density = 0.00013 + weather.air.z * 0.0025 + overcast * 0.00056
	environment.fog_light_color = Color(0.092, 0.15, 0.24).lerp(Color(0.56, 0.67, 0.78), daylight)
	environment.fog_light_color = environment.fog_light_color.lerp(Color(0.64, 0.44, 0.27), weather.air.z)
	environment.volumetric_fog_density = 0.00023 + 0.00051 * warm * daylight + overcast * 0.0017
	environment.volumetric_fog_ambient_inject = 0.065 + overcast * 0.10
	valley_fog.position = Vector3(camera.position.x, minf(camera.position.y + 8.0, local.sea + 150.0), camera.position.z)
	materials.fog.set_shader_parameter("density", (0.005 + 0.023 * warm) * daylight * (0.55 + overcast * 0.96))
	if plunge_spray != null:
		materials.plunge.set_shader_parameter("drift", weather.drift)
		materials.plunge.set_shader_parameter("density", 0.29 + 0.15 * overcast)
	materials.sky.set_shader_parameter("cloudiness", 0.38 + overcast * 0.62)
	materials.sky.set_shader_parameter("drift", weather.drift)


func _apply_quality() -> void:
	var cinematic := forward && quality > 0
	environment.ssao_enabled = cinematic && not software
	environment.ssil_enabled = forward && quality >= 2 && not software
	environment.sdfgi_enabled = cinematic && gi && (not software || audit)
	environment.ssr_enabled = forward && quality >= 2
	environment.volumetric_fog_enabled = cinematic && shafts
	valley_fog.visible = cinematic && shafts
	if plunge_spray != null:
		plunge_spray.amount_ratio = [0.32, 0.66, 1.0][quality]
	sun.light_volumetric_fog_energy = 2.15 if shafts else 0.0
	sun.directional_shadow_max_distance = [135.0, 280.0, 440.0][quality]
	materials.sky.set_shader_parameter("cloud_steps", ([5, 10, 16] if software else [8, 17, 28])[quality])
	materials.terrain.set_shader_parameter("relief", [0.45, 1.0, 1.28][quality])
	radius = 1 if audit || quality == 0 else 2 if quality == 1 else 3
	if active_world:
		_layout(center)


func _adjust_scale(delta: float) -> void:
	if not auto_scale:
		return
	average_frame = lerpf(average_frame, minf(delta * 1000.0, 85.0), 0.035)
	if elapsed < next_scale || not requested.is_empty() || materials.pending != null:
		return
	next_scale = elapsed + 2.1
	var viewport := get_viewport()
	var old := viewport.scaling_3d_scale
	if average_frame > 20.0:
		viewport.scaling_3d_scale = maxf(0.61, old - 0.06)
	elif average_frame < 15.1:
		viewport.scaling_3d_scale = minf(1.0, old + 0.04)


func _move(delta: float) -> void:
	if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
		return
	var x := float(Input.is_physical_key_pressed(KEY_D)) - float(Input.is_physical_key_pressed(KEY_A))
	var z := float(Input.is_physical_key_pressed(KEY_S)) - float(Input.is_physical_key_pressed(KEY_W))
	var intent := Vector3(x, 0, z).normalized()
	var speed := (24.0 if flying else 5.6) * (2.75 if Input.is_physical_key_pressed(KEY_SHIFT) else 1.0)
	camera.position += Basis(Vector3.UP, yaw) * intent * speed * delta
	if flying:
		camera.position.y += delta * speed * (float(Input.is_physical_key_pressed(KEY_SPACE))
			- float(Input.is_physical_key_pressed(KEY_CTRL)))
	else:
		var site := _site()
		var ground := native.ground_height(site.x, site.z) + 1.76
		if camera.position.y <= ground + 0.055:
			up_speed = 5.0 if Input.is_physical_key_pressed(KEY_SPACE) else 0.0
		else:
			up_speed -= delta * 15.0
		camera.position.y = maxf(ground, camera.position.y + up_speed * delta)


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton && event.pressed && event.button_index == MOUSE_BUTTON_LEFT && not audit:
		if editing && Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			_paint(event.position)
		else:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	elif event is InputEventMouseMotion && Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		yaw = wrapf(yaw - event.relative.x * 0.0021, -PI, PI)
		pitch = clampf(pitch - event.relative.y * 0.0021, -1.49, 1.49)
		camera.rotation = Vector3(pitch, yaw, 0)
	elif event is InputEventKey && event.pressed && not event.echo:
		match event.physical_keycode:
			KEY_ESCAPE:
				Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
			KEY_TAB:
				flying = not flying
			KEY_F2:
				bar.visible = not bar.visible
				help.visible = bar.visible
			KEY_F:
				Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _make_ui() -> void:
	var overlay := CanvasLayer.new()
	add_child(overlay)
	bar = PanelContainer.new()
	bar.position = Vector2(16, 16)
	overlay.add_child(bar)
	var rows := VBoxContainer.new()
	bar.add_child(rows)
	var buttons := HBoxContainer.new()
	rows.add_child(buttons)
	seed_input = LineEdit.new()
	seed_input.text = "42"
	seed_input.placeholder_text = "Seed"
	seed_input.custom_minimum_size.x = 83.0
	buttons.add_child(seed_input)
	preset_input = _options(buttons, PRESETS, 9, func(_id: int) -> void: pass)
	_button(buttons, "Generate", func() -> void: _new_world(seed_input.text, preset_input.selected))
	_button(buttons, "Save", _save)
	_button(buttons, "Load", _open_picker)
	var visuals := HBoxContainer.new()
	rows.add_child(visuals)
	_options(visuals, ["Eco", "High", "Ultra"], quality, func(id: int) -> void:
		quality = id
		_apply_quality())
	_options(visuals, ["512 px", "1024 px", "2048 px", "4096 px"], 1, func(id: int) -> void:
		texture_size = 512 << id
		materials.request(local.seed, texture_size))
	_options(visuals, ["Native", "FSR Quality", "FSR Balanced", "Adaptive 60 FPS"], 0, func(id: int) -> void:
		auto_scale = id == 3
		average_frame = 16.7
		next_scale = elapsed + 2.1
		get_viewport().scaling_3d_mode = Viewport.SCALING_3D_MODE_FSR2 if id > 0 && forward else Viewport.SCALING_3D_MODE_BILINEAR
		get_viewport().scaling_3d_scale = [1.0, 0.77, 0.67, 1.0][id])
	_toggle(visuals, "Global illumination", gi, func(value: bool) -> void:
		gi = value
		_apply_quality()).disabled = not forward
	_toggle(visuals, "Sun shafts", shafts, func(value: bool) -> void:
		shafts = value
		_apply_quality()).disabled = not forward
	var controls := HBoxContainer.new()
	rows.add_child(controls)
	_options(controls, Weather.LABELS, 0, func(id: int) -> void: weather.selected = id)
	_toggle(controls, "Day cycle", moving_sun, func(value: bool) -> void: moving_sun = value)
	slider = HSlider.new()
	slider.max_value = 23.99
	slider.step = 0.01
	slider.value = clock
	slider.custom_minimum_size.x = 110
	slider.tooltip_text = "Time of day"
	slider.value_changed.connect(func(value: float) -> void: clock = value)
	controls.add_child(slider)
	status = Label.new()
	rows.add_child(status)
	edit_panel = HBoxContainer.new()
	rows.add_child(edit_panel)
	_toggle(buttons, "Edit", false, func(value: bool) -> void:
		editing = value
		edit_panel.visible = value)
	_options(edit_panel, ["Raise", "Lower", "Flatten", "Smooth", "Terrace", "Paint biome", "Paint vegetation"],
		0, func(id: int) -> void: brush_kind = id)
	_options(edit_panel, ["Ocean", "Beach", "Grassland", "Forest", "Rainforest", "Conifer", "Desert", "Tundra",
		"Snow", "Volcanic", "Savanna", "Wetland", "Meadow", "Birch"], 3, func(id: int) -> void: brush_biome = id)
	var brush_size := SpinBox.new()
	brush_size.min_value = 4
	brush_size.max_value = 128
	brush_size.value = brush_radius
	brush_size.suffix = "m"
	brush_size.value_changed.connect(func(value: float) -> void: brush_radius = value)
	edit_panel.add_child(brush_size)
	_button(edit_panel, "Undo", func() -> void: _history(false))
	_button(edit_panel, "Redo", func() -> void: _history(true))
	edit_panel.hide()
	help = Label.new()
	help.text = "Click to explore   WASD move   Space jump   Tab fly   Esc release   F2 hide UI"
	help.position = Vector2(19, 157)
	help.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.add_child(help)


func _button(parent: Control, title: String, action: Callable) -> void:
	var button := Button.new()
	button.text = title
	button.pressed.connect(action)
	parent.add_child(button)


func _toggle(parent: Control, title: String, value: bool, action: Callable) -> CheckButton:
	var control := CheckButton.new()
	control.text = title
	control.button_pressed = value
	control.toggled.connect(action)
	parent.add_child(control)
	return control


func _options(parent: Control, labels: Array, selected: int, action: Callable) -> OptionButton:
	var control := OptionButton.new()
	for title: String in labels:
		control.add_item(title)
	control.select(selected)
	control.item_selected.connect(action)
	parent.add_child(control)
	return control


func _save() -> void:
	var site := _site()
	var error := native.save_world(ProjectSettings.globalize_path(SAVE), site.x, site.y, site.z, yaw, pitch)
	status.text = "World saved" if error.is_empty() else error


func _open_picker() -> void:
	var dialog := FileDialog.new()
	dialog.access = FileDialog.ACCESS_FILESYSTEM
	dialog.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	dialog.add_filter("*.worldcraft.json", "Worldcraft world")
	dialog.current_dir = ProjectSettings.globalize_path("user://")
	add_child(dialog)
	dialog.file_selected.connect(func(file: String) -> void:
		_begin(native.open_world(file))
		dialog.queue_free())
	dialog.canceled.connect(dialog.queue_free)
	dialog.popup_centered(Vector2i(800, 540))


func _history(redo: bool) -> void:
	if native.undo(redo):
		_refresh_terrain()


func _refresh_terrain() -> void:
	requested.clear()
	horizon_pending = false
	horizon_center = Vector2i(2147483647, 2147483647)
	for tile: Dictionary in tiles.values():
		tile.build = Vector3i(-1, -1, -1)


func _paint(pixel: Vector2) -> void:
	var base := _site()
	var ray := camera.project_ray_normal(pixel)
	for step in range(1, 56):
		var p := base + ray * step * 2.5
		if p.y <= native.ground_height(p.x, p.z):
			if native.brush(p.x, p.z, brush_kind, brush_radius, 5.0, brush_biome):
				_refresh_terrain()
			return


func _scenery_totals() -> Vector3i:
	var totals := Vector3i.ZERO
	for tile: Dictionary in tiles.values():
		var scenery: Vector3i = tile.scenery
		totals.x += scenery.x
		totals.y += scenery.y
		totals.z |= scenery.z
	return totals


func _check_audit(delta: float) -> void:
	var ms := Time.get_ticks_msec() - bench_start
	if ms > 240_000:
		push_error("Native Godot audit timed out while waiting for generated terrain or materials")
		get_tree().quit(1)
		return
	if ms >= next_audit_report:
		print("Waiting for ", tiles.size(), "/", wanted.size(), " tiles, horizon=", horizon != null,
			", texture=", materials.active_side, "/", texture_size, ", pending=", requested.size())
		next_audit_report = ms + 6000
	if tiles.size() < wanted.size() || horizon == null || materials.active_side != texture_size:
		return
	settle_frames += 1
	if settle_frames > 12:
		settled_ms.append(delta * 1000.0)
	if settle_frames == 32:
		if not audit_image.is_empty():
			var image := get_viewport().get_texture().get_image()
			if image == null || image.save_png(audit_image) != OK:
				push_error("Could not save Godot audit screenshot")
				get_tree().quit(1)
				return
		settled_ms.sort()
		var scenery := _scenery_totals()
		var kinds := 0
		for bit in 18:
			kinds += (scenery.z >> bit) & 1
		print("WORLDCRAFT_AUDIT ", JSON.stringify({
			"renderer": RenderingServer.get_current_rendering_method(),
			"adapter": RenderingServer.get_video_adapter_name(),
			"seed": local.seed_text, "terrain_tiles": tiles.size(), "horizon": horizon != null,
			"scenery_objects": scenery.x, "scenery_batches": scenery.y, "scenery_kinds": kinds,
			"river": river != null, "spray": plunge_spray != null, "texture_side": materials.active_side,
			"gi": environment.sdfgi_enabled, "volumetrics": environment.volumetric_fog_enabled,
			"adapter_ray_query": adapter_rays,
			"weather": Weather.LABELS[weather.condition], "median_frame_ms": settled_ms[10],
			"p95_frame_ms": settled_ms[18],
		}))
		get_tree().quit()
