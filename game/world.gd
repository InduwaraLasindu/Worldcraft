extends Node3D

const Materials = preload("res://materials.gd")
const Nature = preload("res://nature.gd")
const Weather = preload("res://weather.gd")
const PRESETS := ["Temperate wilderness", "Alpine mountains", "Tropical highlands", "Desert and canyons",
	"Arctic wilderness", "Archipelago", "Grassland and savanna", "Volcanic islands", "Tidal estuary",
	"River valley", "Empty canvas"]
const TILE := 128.0
const SAVE := "user://worldcraft.worldcraft.json"
const START_SEED := "421"
const START_PRESET := 0
const UHD_OUTPUT := Vector2i(3840, 2160)

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
var lake_nodes := {}
var river: MeshInstance3D
var plunge_spray: GPUParticles3D
var horizon: MeshInstance3D
var distant_trees := 0
var horizon_center := Vector2i(2147483647, 2147483647)
var horizon_coverage := Vector2i.ZERO
var requested_coverage := Vector2i.ZERO
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
var clock := 8.4
var latitude := Vector2.from_angle(deg_to_rad(32.0))
var declination := Vector2.RIGHT
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
var last_audit_usec := 0
var settled_ms: Array[float] = []
var next_audit_report := 0
var audit := false
var audit_image := ""
var audit_uhd := false
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
var display_mode := 0
var windowed_rect := Rect2i()
var windowed_maximized := false
var render_picker: OptionButton
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
	sun.shadow_bias = 0.056
	sun.shadow_normal_bias = 1.20
	sun.directional_shadow_split_1 = 0.065
	sun.directional_shadow_split_2 = 0.19
	sun.directional_shadow_split_3 = 0.43
	var world_environment := WorldEnvironment.new()
	environment.background_mode = Environment.BG_SKY
	environment.fog_sky_affect = 0.025
	environment.volumetric_fog_sky_affect = 0.09
	var sky := Sky.new()
	sky.sky_material = materials.sky
	sky.radiance_size = Sky.RADIANCE_SIZE_64 if software else Sky.RADIANCE_SIZE_128
	sky.process_mode = Sky.PROCESS_MODE_INCREMENTAL
	environment.sky = sky
	environment.tonemap_mode = Environment.TONE_MAPPER_AGX
	environment.tonemap_agx_white = 8.3
	environment.tonemap_agx_contrast = 1.43
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	environment.ambient_light_sky_contribution = 0.20
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	environment.fog_enabled = true
	environment.glow_blend_mode = Environment.GLOW_BLEND_MODE_ADDITIVE
	environment.glow_hdr_threshold = 1.65
	environment.glow_intensity = 0.085
	environment.glow_strength = 0.78
	environment.ssao_radius = 1.35
	environment.ssao_intensity = 1.50
	environment.ssil_radius = 2.9
	environment.ssil_intensity = 0.59
	environment.sdfgi_min_cell_size = 0.62
	environment.sdfgi_use_occlusion = true
	environment.sdfgi_bounce_feedback = 0.21
	environment.sdfgi_energy = 0.78
	if software:
		environment.sdfgi_cascades = 2
		RenderingServer.environment_set_sdfgi_ray_count(RenderingServer.ENV_SDFGI_RAY_COUNT_4)
	environment.volumetric_fog_length = 285.0
	environment.volumetric_fog_detail_spread = 2.1
	environment.volumetric_fog_anisotropy = 0.61
	environment.volumetric_fog_gi_inject = 0.12
	world_environment.environment = environment
	add_child(world_environment)
	valley_fog = FogVolume.new()
	valley_fog.shape = RenderingServer.FOG_VOLUME_SHAPE_BOX
	valley_fog.size = Vector3(690, 39, 690)
	valley_fog.material = materials.fog
	add_child(valley_fog)
	add_child(weather)
	_make_ui()
	_set_render_mode(0)
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
		audit_uhd = "--uhd" in args
		if audit_uhd && not _set_display_mode(2):
			push_error("Native 4K audit requires a display at least 3840 × 2160")
			get_tree().quit(1)
			return
		if audit_uhd && software:
			get_viewport().disable_3d = true
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
			var direction := _sun_direction()
			direction.y = 0.065
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
		_new_world(START_SEED, START_PRESET)


func _arg(values: PackedStringArray, name: String, fallback: String) -> String:
	var index := values.find(name)
	return values[index + 1] if index >= 0 && index + 1 < values.size() else fallback


func _new_world(seed_text: String, preset: int) -> void:
	var info := native.create_world(seed_text, preset)
	_begin(info)
	if preset == START_PRESET && not info.has("error"):
		# Open the woodland at an oblique angle to the seed's sunlight.
		var direction := _sun_direction().rotated(Vector3.UP, 0.43)
		yaw = atan2(-direction.x, -direction.z)
		pitch = deg_to_rad(-5.0)
		camera.rotation = Vector3(pitch, yaw, 0.0)


func _begin(info: Dictionary) -> void:
	if info.has("error"):
		status.text = info.error
		return
	for child in land.get_children():
		land.remove_child(child)
		child.queue_free()
	tiles.clear()
	lake_nodes.clear()
	requested.clear()
	order.clear()
	wanted.clear()
	ocean = null
	river = null
	plunge_spray = null
	horizon = null
	distant_trees = 0
	horizon_pending = false
	center = Vector2i(2147483647, 2147483647)
	horizon_center = center
	local = info
	var sunlight := RandomNumberGenerator.new()
	sunlight.seed = int(info.seed) ^ 0x273c9e41
	latitude = Vector2.from_angle(deg_to_rad(sunlight.randf_range(28.0, 42.0)))
	declination = Vector2.from_angle(deg_to_rad(sunlight.randf_range(-7.0, 12.0)))
	origin = Vector2i(floori(info.x / TILE), floori(info.z / TILE))
	camera.position = Vector3(info.x - origin.x * TILE, info.y, info.z - origin.y * TILE)
	yaw = info.yaw
	pitch = info.pitch
	camera.rotation = Vector3(pitch, yaw, 0)
	up_speed = 0.0
	climate_update = 0.0
	climate.clear()
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
	_sync_lakes(focus)
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


func _sync_lakes(focus: Vector2i) -> void:
	var active := {}
	for basin: Dictionary in native.lakes(focus.x * TILE, focus.y * TILE):
		var key: Vector2i = basin.cell
		if lake_nodes.has(key):
			active[key] = lake_nodes[key]
			continue
		var surface := _mesh(basin, materials.lake)
		surface.position = Vector3(basin.x - origin.x * TILE, basin.y, basin.z - origin.y * TILE)
		surface.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		surface.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		land.add_child(surface)
		active[key] = surface
	for key: Vector2i in lake_nodes:
		if not active.has(key):
			lake_nodes[key].queue_free()
	lake_nodes = active


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
			distant_trees = nature.attach_distant(horizon, data.nature)
			horizon_center = key
			horizon_coverage = requested_coverage
		else:
			var finished = requested.get(key, null)
			requested.erase(key)
			if wanted.has(key) && finished != null && data.lod == wanted[key].x && finished.y == wanted[key].y && finished.z == _edges(key, data.lod):
				var tile := _mesh(data, materials.terrain)
				tile.position = Vector3((key.x - origin.x) * TILE, 0.0, (key.y - origin.y) * TILE)
				land.add_child(tile)
				var scenery := nature.attach(tile, data.nature, finished.y)
				var turf := nature.attach_turf(tile, data.turf, 47.0 if software else 112.0)
				if tiles.has(key):
					tiles[key].node.queue_free()
				tiles[key] = {"node": tile, "build": finished, "scenery": scenery, "turf": turf}
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
	var coverage := Vector2i(radius, 12288 if quality == 2 else 8192)
	if requested.is_empty() && not horizon_pending && (horizon_center != center || horizon_coverage != coverage):
		horizon_pending = native.request_horizon(center.x, center.y, coverage.x, coverage.y)
		if horizon_pending:
			requested_coverage = coverage


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
			var viewport := get_viewport()
			var pixels := Vector2i(viewport.get_visible_rect().size)
			status.text = "%s  ·  %s  ·  %02d:%02d  ·  %d × %d at %d%%  ·  %d objects  ·  %d FPS" % [
				climate.get("biome", "Exploring"), Weather.LABELS[weather.condition], int(clock),
				int(fposmod(clock, 1.0) * 60.0), pixels.x, pixels.y, roundi(viewport.scaling_3d_scale * 100.0),
				_scenery_totals().x, Engine.get_frames_per_second()]
	weather.tick(dt, camera.position, climate, site, local.sea, quality)
	materials.animate(elapsed, weather.wind, weather.wet, weather.snow, weather.dust,
		Vector3(fposmod(origin.x * TILE, 64000.0), 0.0, fposmod(origin.y * TILE, 64000.0)))
	_sunlight(dt)
	if audit:
		_check_audit()


func _sun_direction() -> Vector3:
	var hour := (clock - 12.0) * TAU / 24.0
	var meridian := declination.x * cos(hour)
	return Vector3(-declination.x * sin(hour), latitude.y * declination.y + latitude.x * meridian,
		latitude.y * meridian - latitude.x * declination.y)


func _sunlight(_dt: float) -> void:
	var sun_axis := _sun_direction()
	var elevation := maxf(0.0, sun_axis.y)
	sun.look_at_from_position(sun_axis * 100.0, Vector3.ZERO)
	var daylight := smoothstep(-0.15, 0.22, sun_axis.y)
	var warm := exp(-elevation * 5.3)
	var cloud := clampf(weather.air.x * 1.30 + weather.air.y * 1.20 + weather.air.z * 0.72, 0.0, 1.0)
	# A longer path through the atmosphere dims and warms the direct beam.
	var airmass := 1.0 / (elevation + 0.15)
	var sun_through_air := exp(-0.14 * (airmass - 0.87))
	var sun_through_cloud := exp(-2.25 * cloud * cloud)
	var sun_energy := 2.48 * sun_through_air * sun_through_cloud * smoothstep(-0.015, 0.085, sun_axis.y)
	sun.light_color = Color(1.0, 0.97, 0.90).lerp(Color(1.0, 0.65, 0.40), warm * 0.83)
	sun.light_energy = sun_energy + weather.flash * 1.1
	environment.ambient_light_color = Color(0.30, 0.39, 0.57).lerp(Color(0.76, 0.82, 0.91), daylight)
	environment.ambient_light_energy = 0.033 + daylight * (0.155 + 0.19 * cloud) + weather.flash * 0.29
	environment.tonemap_exposure = lerpf(1.28, 1.04, daylight) + cloud * 0.13
	environment.fog_aerial_perspective = (0.63 - 0.34 * cloud) * (1.0 - weather.air.z * 0.85)
	environment.fog_density = 0.000124 + 0.00010 * warm * daylight + weather.air.z * 0.0028 + cloud * 0.00057
	environment.fog_light_color = Color(0.075, 0.115, 0.18).lerp(Color(0.57, 0.67, 0.76), daylight)
	environment.fog_light_color = environment.fog_light_color.lerp(Color(0.65, 0.45, 0.26), weather.air.z)
	environment.fog_sun_scatter = 0.055 * daylight * (1.0 - cloud)
	environment.volumetric_fog_density = 0.00010 + 0.00021 * warm * daylight + cloud * 0.0015
	environment.volumetric_fog_ambient_inject = 0.048 + cloud * 0.096
	var ground: float = maxf(local.sea, climate.get("height", camera.position.y - 1.76))
	valley_fog.position = Vector3(camera.position.x, ground + 6.4, camera.position.z)
	var humidity: float = climate.get("humidity", 0.56)
	var bank := (0.0019 + 0.0135 * warm * humidity) * daylight * (0.46 + cloud * 0.86)
	materials.fog.set_shader_parameter("density", bank * (2.1 if river != null else 1.0))
	var water_sky := Vector3(0.019, 0.027, 0.046).lerp(Vector3(0.29, 0.43, 0.63), daylight * (1.0 - cloud * 0.37))
	materials.sea.set_shader_parameter("horizon_light", water_sky)
	materials.lake.set_shader_parameter("horizon_light", water_sky)
	if plunge_spray != null:
		materials.plunge.set_shader_parameter("drift", weather.drift)
		materials.plunge.set_shader_parameter("density", 0.26 + 0.15 * cloud)
	materials.sky.set_shader_parameter("cloudiness", 0.38 + cloud * 0.62)
	materials.sky.set_shader_parameter("drift", weather.drift)


func _apply_quality() -> void:
	materials.turf.set_shader_parameter("reach", ([30.0, 42.0, 47.0] if software else [42.0, 82.0, 112.0])[quality])
	for surface in [materials.sea, materials.lake]:
		surface.set_shader_parameter("reflection_steps", ([0, 9, 14] if software else [0, 13, 20])[quality])
	var cinematic := forward && quality > 0
	environment.ssao_enabled = cinematic && not software
	environment.ssil_enabled = forward && quality >= 2 && not software
	environment.sdfgi_enabled = cinematic && gi && (not software || audit)
	environment.ssr_enabled = forward && quality >= 2
	environment.volumetric_fog_enabled = cinematic && shafts
	environment.glow_enabled = cinematic
	valley_fog.visible = cinematic && shafts
	if plunge_spray != null:
		plunge_spray.amount_ratio = [0.32, 0.66, 1.0][quality]
	sun.light_volumetric_fog_energy = 2.0 if shafts else 0.0
	sun.directional_shadow_max_distance = [125.0, 255.0, 395.0][quality]
	if forward:
		RenderingServer.directional_soft_shadow_filter_set_quality(
			RenderingServer.SHADOW_QUALITY_SOFT_LOW if software else
			[RenderingServer.SHADOW_QUALITY_SOFT_LOW, RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
				RenderingServer.SHADOW_QUALITY_SOFT_HIGH][quality])
		RenderingServer.environment_set_volumetric_fog_volume_size(
			56 if software else [48, 80, 112][quality], 56 if software else [40, 64, 72][quality])
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
	if average_frame > 20.0 && old > 0.61:
		viewport.scaling_3d_scale = maxf(0.61, old - 0.06)
	elif average_frame < 15.1 && old < 1.0:
		viewport.scaling_3d_scale = minf(1.0, old + 0.04)


func _set_render_mode(id: int) -> void:
	auto_scale = id == 3
	average_frame = 16.7
	next_scale = elapsed + 2.1
	var viewport := get_viewport()
	var fsr := id != 0 && forward
	viewport.scaling_3d_mode = Viewport.SCALING_3D_MODE_FSR2 if fsr else Viewport.SCALING_3D_MODE_BILINEAR
	viewport.scaling_3d_scale = [1.0, 0.77, 0.67, 1.0][id]
	viewport.fsr_sharpness = 0.14
	viewport.texture_mipmap_bias = -0.15 if id == 0 else 0.0
	viewport.anisotropic_filtering_level = Viewport.ANISOTROPY_8X
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA
	if forward:
		viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED if fsr else Viewport.SCREEN_SPACE_AA_SMAA


func _supports_uhd(screen: Vector2i) -> bool:
	return screen.x >= UHD_OUTPUT.x && screen.y >= UHD_OUTPUT.y


func _set_display_mode(id: int) -> bool:
	if id == display_mode:
		return true
	var window := get_window()
	var screen_size := DisplayServer.screen_get_size(window.current_screen)
	if id == 2 && not _supports_uhd(screen_size):
		return false
	if display_mode == 0 && id != 0:
		windowed_rect = Rect2i(window.position, window.size)
		windowed_maximized = window.mode == Window.MODE_MAXIMIZED
	window.unresizable = false
	window.borderless = false
	match id:
		0:
			window.mode = Window.MODE_WINDOWED
			window.size = windowed_rect.size
			window.position = windowed_rect.position
			if windowed_maximized:
				window.mode = Window.MODE_MAXIMIZED
		1:
			window.size = screen_size
			window.mode = Window.MODE_FULLSCREEN
		2:
			_set_render_mode(0)
			render_picker.select(0)
			window.mode = Window.MODE_WINDOWED
			window.borderless = screen_size != UHD_OUTPUT
			window.size = UHD_OUTPUT
			if screen_size == UHD_OUTPUT:
				window.mode = Window.MODE_FULLSCREEN
			else:
				window.position = DisplayServer.screen_get_position(window.current_screen) + (screen_size - UHD_OUTPUT) / 2
				window.unresizable = true
	render_picker.disabled = id == 2
	display_mode = id
	return true


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
	seed_input.text = START_SEED
	seed_input.placeholder_text = "Seed"
	seed_input.custom_minimum_size.x = 83.0
	buttons.add_child(seed_input)
	preset_input = _options(buttons, PRESETS, START_PRESET, func(_id: int) -> void: pass)
	_button(buttons, "Generate", func() -> void: _new_world(seed_input.text, preset_input.selected))
	_button(buttons, "Save", _save)
	_button(buttons, "Load", _open_picker)
	var visuals := HBoxContainer.new()
	rows.add_child(visuals)
	_options(visuals, ["Eco", "High", "Ultra"], quality, func(id: int) -> void:
		quality = id
		_apply_quality())
	_options(visuals, ["Materials 512 px", "Materials 1024 px", "Materials 2048 px", "Materials 4096 px"], 1, func(id: int) -> void:
		texture_size = 512 << id
		materials.request(local.seed, texture_size))
	_toggle(visuals, "Global illumination", gi, func(value: bool) -> void:
		gi = value
		_apply_quality()).disabled = not forward
	_toggle(visuals, "Sun shafts", shafts, func(value: bool) -> void:
		shafts = value
		_apply_quality()).disabled = not forward
	var display := HBoxContainer.new()
	rows.add_child(display)
	var outputs := _options(display, ["Windowed", "Desktop fullscreen", "Native 4K (3840 × 2160)"], 0,
		func(id: int) -> void:
			if not _set_display_mode(id):
				status.text = "Native 4K requires a 3840 × 2160 desktop or larger.")
	outputs.get_popup().about_to_popup.connect(func() -> void:
		outputs.select(display_mode)
		outputs.set_item_disabled(2, not _supports_uhd(DisplayServer.screen_get_size(get_window().current_screen))))
	outputs.tooltip_text = "Native 4K requires a 3840 × 2160 desktop or larger and uses 100% render scale."
	render_picker = _options(display, ["Native render (100%)", "FSR Ultra Quality", "FSR Quality", "Adaptive"], 0, _set_render_mode)
	render_picker.set_item_disabled(1, not forward)
	render_picker.set_item_disabled(2, not forward)
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
	help.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.add_child(help)
	bar.resized.connect(func() -> void: help.position = Vector2(19, bar.position.y + bar.size.y + 7.0))


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


func _check_audit() -> void:
	var now := Time.get_ticks_usec()
	var ms := now / 1000.0 - bench_start
	if ms > (480_000 if audit_uhd else 240_000):
		push_error("Native Godot audit timed out while waiting for generated terrain or materials")
		get_tree().quit(1)
		return
	if ms >= next_audit_report:
		print("Waiting for ", tiles.size(), "/", wanted.size(), " tiles, horizon=", horizon != null,
			", texture=", materials.active_side, "/", texture_size, ", pending=", requested.size())
		next_audit_report = ms + 6000
	if tiles.size() < wanted.size() || horizon == null || materials.active_side != texture_size:
		return
	if get_viewport().disable_3d:
		get_viewport().disable_3d = false
	var warmup := 4 if software && audit_uhd else 8
	var end := 8 if software && audit_uhd else 20 if software else 32
	settle_frames += 1
	if settle_frames > warmup:
		settled_ms.append((now - last_audit_usec) * 0.001)
	last_audit_usec = now
	if settle_frames == end:
		var viewport := get_viewport()
		var pixels := Vector2i(viewport.get_visible_rect().size)
		var image: Image
		if not audit_image.is_empty():
			image = viewport.get_texture().get_image()
			if image != null:
				pixels = image.get_size()
		if audit_uhd && (pixels != UHD_OUTPUT || not is_equal_approx(viewport.scaling_3d_scale, 1.0)):
			push_error("Native 3840 × 2160 audit captured %s at %s render scale" % [pixels, viewport.scaling_3d_scale])
			get_tree().quit(1)
			return
		if not audit_image.is_empty() && (image == null || image.save_png(audit_image) != OK):
			push_error("Could not save Godot audit screenshot")
			get_tree().quit(1)
			return
		settled_ms.sort()
		var scenery := _scenery_totals()
		var kinds := 0
		var turf := 0
		for entry: Dictionary in tiles.values():
			turf += entry.turf
		for bit in 18:
			kinds += (scenery.z >> bit) & 1
		print("WORLDCRAFT_AUDIT ", JSON.stringify({
			"renderer": RenderingServer.get_current_rendering_method(),
			"adapter": RenderingServer.get_video_adapter_name(),
			"output_width": pixels.x, "output_height": pixels.y, "render_scale": viewport.scaling_3d_scale,
			"seed": local.seed_text, "terrain_tiles": tiles.size(), "horizon": horizon != null,
			"scenery_objects": scenery.x, "scenery_batches": scenery.y, "scenery_kinds": kinds,
			"grass_tufts": turf, "inland_lakes": lake_nodes.size(), "distant_trees": distant_trees,
			"river": river != null, "spray": plunge_spray != null, "texture_side": materials.active_side,
			"gi": environment.sdfgi_enabled, "volumetrics": environment.volumetric_fog_enabled,
			"sun_energy": sun.light_energy, "sun_elevation": _sun_direction().y,
			"exposure": environment.tonemap_exposure,
			"adapter_ray_query": adapter_rays,
			"weather": Weather.LABELS[weather.condition], "median_frame_ms": settled_ms[settled_ms.size() >> 1],
			"p95_frame_ms": settled_ms[ceili(settled_ms.size() * 0.95) - 1],
		}))
		get_tree().quit()
