extends SceneTree

func _initialize() -> void:
	call_deferred("_check")


func _check() -> void:
	if not ClassDB.class_exists("WorldcraftNative"):
		push_error("Rust bridge was not registered")
		quit(1)
		return
	var world: Object = ClassDB.instantiate("WorldcraftNative")
	var info: Dictionary = world.create_world("42", 9)
	assert(info.seed_text == "42", "numeric seed changed")
	assert(not world.river().is_empty(), "River Valley mesh is missing")
	var x := floori(info.x / 128.0)
	var z := floori(info.z / 128.0)
	assert(world.request_tile(x, z, 0, 1, Vector4i.ZERO))
	var tile := await _poll(world)
	assert(not tile.is_empty() && tile.positions.size() > 1000)
	assert(tile.indices.size() >= 32 * 32 * 6)
	assert(tile.nature.size() % 7 == 0)
	assert(world.request_tile(x, z, 0, 1, Vector4i.ZERO))
	var again := await _poll(world)
	assert(tile.positions == again.positions && tile.nature == again.nature)
	assert(world.request_tile(x, z, 0, 1, Vector4i.ZERO))
	var desert: Dictionary = world.create_world("326", 3)
	var desert_x := floori(desert.x / 128.0)
	var desert_z := floori(desert.z / 128.0)
	assert(world.request_tile(desert_x, desert_z, 0, 1, Vector4i.ZERO))
	var changed := await _poll(world)
	assert(not changed.is_empty() && changed.positions != tile.positions)
	assert(world.river().is_empty())
	var sample_x: float = floorf(desert.x / 4.0) * 4.0
	var sample_z: float = floorf(desert.z / 4.0) * 4.0
	var before: float = world.ground_height(sample_x, sample_z)
	assert(world.brush(sample_x, sample_z, 0, 24.0, 5.0, 3))
	assert(is_equal_approx(world.ground_height(sample_x, sample_z), before + 5.0))
	assert(world.undo(false) && is_equal_approx(world.ground_height(sample_x, sample_z), before))
	assert(world.undo(true) && is_equal_approx(world.ground_height(sample_x, sample_z), before + 5.0))
	var file := ProjectSettings.globalize_path("user://integration.worldcraft.json")
	assert(world.save_world(file, desert.x, desert.y, desert.z, 0.12, -0.15) == "")
	var loaded: Dictionary = world.open_world(file)
	assert(loaded.seed_text == "326" && is_equal_approx(loaded.yaw, 0.12))
	assert(is_equal_approx(world.ground_height(sample_x, sample_z), before + 5.0))
	DirAccess.remove_absolute(file)
	print("WORLDCRAFT_INTEGRATION_OK derived mesh, seed, river, stale chunks and saved world")
	quit()


func _poll(world: Object) -> Dictionary:
	for unused in 150:
		var data: Dictionary = world.poll_chunk()
		if not data.is_empty():
			return data
		await create_timer(0.02).timeout
	return {}
