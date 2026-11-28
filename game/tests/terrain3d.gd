extends SceneTree
## Opt-in integration fixture for the official Terrain3D 1.1.0 addon.

const SIDE := 1024
const HOLE := Rect2i(504, 505, 18, 18)


func _initialize() -> void:
	call_deferred("_check")


func _check() -> void:
	if not ClassDB.class_exists("Terrain3D") || not ClassDB.class_exists("WorldcraftTerrain3D"):
		_fail("Terrain3D 1.1.0 and the Worldcraft GDExtension must be installed")
		return
	var bridge: Object = ClassDB.instantiate("WorldcraftTerrain3D")
	var terrain: Node3D = ClassDB.instantiate("Terrain3D")
	terrain.call("set_free_editor_textures", false)
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = Vector3(570, 104, 635)
	camera.look_at(Vector3(505, 44, 504))
	camera.make_current()
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-40.0, -28.0, 0.0)
	sun.shadow_enabled = true
	root.add_child(sun)
	terrain.call("set_camera", camera)
	root.add_child(terrain)
	terrain.call("change_region_size", SIDE)
	terrain.call("set_vertex_spacing", 1.0)
	terrain.call("set_collision_mode", ClassDB.class_get_integer_constant("Terrain3DCollision", "FULL_GAME"))
	var material: Object = terrain.call("get_material")
	material.call("set_world_background", ClassDB.class_get_integer_constant("Terrain3DMaterial", "NONE"))
	material.call("set_auto_shader_enabled", false)
	material.call("set_dual_scaling_enabled", true)
	if DisplayServer.get_name() != "headless":
		var assets: Object = terrain.call("get_assets")
		_add_texture(assets, 0, Color(0.21, 0.30, 0.12), Color(0.43, 0.48, 0.27))
		_add_texture(assets, 1, Color(0.31, 0.26, 0.20), Color(0.58, 0.52, 0.44))
	var heights := PackedFloat32Array()
	var controls := PackedInt32Array()
	var colors := PackedByteArray()
	heights.resize(SIDE * SIDE)
	controls.resize(SIDE * SIDE)
	colors.resize(SIDE * SIDE * 4)
	colors.fill(255)
	var ramp := PackedInt32Array()
	for blend in range(256):
		ramp.append(bridge.call("pack_control", 0, 1, blend, 0, 0, false, true, false))
	for z in range(SIDE):
		for x in range(SIDE):
			var index := z * SIDE + x
			heights[index] = _height(x, z)
			var blend := clampi(int((x - 280) * 255.0 / 430.0), 0, 255)
			controls[index] = ramp[blend] | (4 if HOLE.has_point(Vector2i(x, z)) else 0)
			colors[index * 4 + 3] = 128
	# The all-bits fixture value at the far corner would be a NaN as a numeric float.
	controls[SIDE * SIDE - 1] = bridge.call("pack_control", 31, 31, 255, 15, 7, true, true, true)
	var error: String = bridge.call("upload_region", terrain, Vector2i.ZERO, heights, controls, colors)
	if not error.is_empty():
		_fail(error)
		return
	var data: Object = terrain.call("get_data")
	# Sample inside the height cells since the physics backend can miss a ray on a vertex.
	var solid := Vector3(468.32, 0, 511.41)
	var hole := Vector3(513.32, 0, 514.41)
	var region: Object = data.call("get_region", Vector2i.ZERO)
	var control_map: Image = region.call("get_control_map")
	var bits := control_map.get_data().decode_u32((SIDE * SIDE - 1) * 4)
	var blend_id: Vector3 = data.call("get_texture_id", solid)
	if bits != 0xffffff87 || blend_id.x != 0 || blend_id.y != 1 || blend_id.z <= 0.3 || blend_id.z >= 0.7:
		_fail("Terrain3D did not preserve control words or material blending")
		return
	if not data.call("get_control_hole", hole) || not is_nan(data.call("get_surface_height", hole)):
		_fail("Terrain3D did not apply the surface hole")
		return
	await physics_frame
	await physics_frame
	var space := terrain.get_world_3d().direct_space_state
	var ground := space.intersect_ray(_ray(solid))
	var surface: float = data.call("get_surface_height", solid)
	if ground.is_empty() || absf(ground.position.y - surface) > 0.15:
		_fail("Physics ray did not hit the uploaded terrain at the expected height")
		return
	if not space.intersect_ray(_ray(hole)).is_empty():
		_fail("Terrain collision was present inside the punched hole")
		return
	var args := OS.get_cmdline_user_args()
	if args.size() == 2 && args[0] == "--image":
		await RenderingServer.frame_post_draw
		var capture := root.get_texture().get_image()
		if capture == null || capture.save_png(args[1]) != OK:
			_fail("Could not write the Terrain3D probe capture")
			return
	print("WORLDCRAFT_TERRAIN3D_OK 1024m region, material blend, raw control bits, and solid/hole collision")
	quit()


func _height(x: int, z: int) -> float:
	var position := Vector2(x - 480, z - 470) / 295.0
	return 8.0 + 61.0 * exp(-position.length_squared()) + 2.8 * sin(x * 0.035) * cos(z * 0.029)


func _ray(at: Vector3) -> PhysicsRayQueryParameters3D:
	return PhysicsRayQueryParameters3D.create(at + Vector3.UP * 150.0, at - Vector3.UP * 50.0)


func _add_texture(assets: Object, id: int, low: Color, high: Color) -> void:
	var asset: Object = ClassDB.instantiate("Terrain3DTextureAsset")
	var base := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	var normal := Image.create_empty(64, 64, false, Image.FORMAT_RGBA8)
	var noise := FastNoiseLite.new()
	noise.seed = 18461 + id
	noise.frequency = 0.14
	for z in range(64):
		for x in range(64):
			var grain := noise.get_noise_2d(x, z) * 0.5 + 0.5
			var tint := low.lerp(high, grain)
			tint.a = grain
			base.set_pixel(x, z, tint)
			normal.set_pixel(x, z, Color(0.5, 0.5, 1.0, 0.89 - grain * 0.16))
	base.generate_mipmaps()
	normal.generate_mipmaps()
	asset.call("set_albedo_texture", ImageTexture.create_from_image(base))
	asset.call("set_normal_texture", ImageTexture.create_from_image(normal))
	assets.call("set_texture_asset", id, asset)


func _fail(reason: String) -> void:
	push_error(reason)
	quit(1)
