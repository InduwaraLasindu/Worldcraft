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
	var pine: Dictionary = world.tree_meshes(1, 2, false)
	var distant_pine: Dictionary = world.tree_meshes(1, 2, true)
	assert(pine.leaves.indices.size() > distant_pine.leaves.indices.size())
	assert(pine.wood.positions == world.tree_meshes(1, 2, false).wood.positions)
	assert(world.tree_atlas().size() == 1024 * 512 * 4)
	assert(world.tree_view(1).size() == 6)
	var network: Dictionary = world.river()
	assert(not network.is_empty(), "River Valley mesh is missing")
	assert(network.plunge.is_finite() && network.plunge_half_width > 0.0)
	assert(network.plunge.y > world.ground_height(network.plunge.x, network.plunge.z), "Spray must spawn above the channel bed")
	var aerated := false
	for color: Color in network.colors:
		aerated = aerated || color.r > 0.8
	assert(aerated, "The waterfall is missing its plunge-pool turbulence mask")
	var x := floori(info.x / 128.0)
	var z := floori(info.z / 128.0)
	assert(world.request_tile(x, z, 0, 0, Vector4i.ZERO))
	var tile := await _poll(world)
	assert(not tile.is_empty() && tile.positions.size() > 1000)
	assert(tile.indices.size() >= 32 * 32 * 6)
	var objects := 0
	for batch: Dictionary in tile.nature:
		var buffer: PackedFloat32Array = batch.buffer
		assert(buffer.size() > 0 && buffer.size() % 16 == 0)
		for at in range(0, buffer.size(), 16):
			assert(buffer[at + 3] >= 0.0 && buffer[at + 3] < 128.0)
			assert(is_finite(buffer[at + 7]))
			assert(buffer[at + 11] >= 0.0 && buffer[at + 11] < 128.0)
		objects += buffer.size() >> 4
	assert(objects > 0, "Scenery worker returned no renderable objects")
	var tufts := 0
	for patch: PackedFloat32Array in tile.turf:
		assert(patch.size() % 16 == 0)
		tufts += patch.size() >> 4
	assert(tufts > 200 && tile.turf.size() <= 16)
	assert(world.request_tile(x, z, 0, 0, Vector4i.ZERO))
	var again := await _poll(world)
	assert(tile.positions == again.positions && tile.nature == again.nature && tile.turf == again.turf)
	assert(world.request_tile(x, z, 0, 1, Vector4i.ZERO))
	var desert: Dictionary = world.create_world("326", 3)
	var desert_x := floori(desert.x / 128.0)
	var desert_z := floori(desert.z / 128.0)
	assert(world.request_tile(desert_x, desert_z, 0, 1, Vector4i.ZERO))
	var changed := await _poll(world)
	assert(not changed.is_empty() && changed.positions != tile.positions)
	var desert_sand := false
	for substrate: Vector2 in changed.uv:
		desert_sand = desert_sand || substrate.x > 0.25
	assert(desert_sand, "The desert mesh is missing its sand material weights")
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
	var alpine: Dictionary = world.create_world("324", 1)
	var lakes: Array = world.lakes(alpine.x, alpine.z)
	assert(lakes.size() > 1, "The alpine region must contain multiple seeded lakes")
	for basin: Dictionary in lakes:
		assert(basin.positions.size() == 129 && basin.indices.size() == 384)
		assert(world.ground_height(basin.x, basin.z) < basin.y)
	print("WORLDCRAFT_INTEGRATION_OK derived mesh, seed, river, stale chunks and saved world")
	quit()


func _poll(world: Object) -> Dictionary:
	for unused in 150:
		var data: Dictionary = world.poll_chunk()
		if not data.is_empty():
			return data
		await create_timer(0.02).timeout
	return {}
