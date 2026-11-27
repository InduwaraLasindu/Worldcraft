extends RefCounted
## Procedural geometry shared across world-aligned batches.

const TREES := [0, 1, 2, 6, 7]
var material_library: RefCounted
var seed_value: int
var prototypes := {}


func reset(materials: RefCounted, new_seed: int) -> void:
	material_library = materials
	seed_value = new_seed
	prototypes.clear()


func attach(parent: Node3D, packet: PackedFloat32Array, detail: int) -> void:
	var groups := {}
	for i in range(0, packet.size(), 7):
		var species := int(packet[i + 3])
		var variant := int(packet[i + 6]) % (4 if detail == 0 else 2)
		if species not in TREES and species != 3:
			# Four tall/short understory silhouettes share the grass shader.
			variant = species % 4
			species = 4
		var key := Vector2i(species, variant)
		if not groups.has(key):
			groups[key] = []
		var scale := packet[i + 4]
		var yaw := packet[i + 5]
		var squash := Vector3(scale, scale * (0.86 + 0.23 * sin(yaw * 1.16 + variant)), scale)
		var transform := Transform3D(Basis(Vector3.UP, yaw).scaled_local(squash),
			Vector3(packet[i], packet[i + 1], packet[i + 2]))
		var tint := Color(fposmod(yaw * 0.511, 1.0), fposmod(yaw * 0.737, 1.0),
			0.39 + 0.37 * sin(yaw + variant) + 0.17 * scale, 1.0)
		groups[key].append([transform, tint])
	for key: Vector2i in groups:
		var entries: Array = groups[key]
		for prototype: Mesh in meshes(key, detail):
			var batch := MultiMesh.new()
			batch.transform_format = MultiMesh.TRANSFORM_3D
			batch.use_custom_data = true
			batch.mesh = prototype
			batch.instance_count = entries.size()
			var buffer := PackedFloat32Array()
			buffer.resize(entries.size() * 16)
			for index in entries.size():
				var entry: Transform3D = entries[index][0]
				var color: Color = entries[index][1]
				var at := index * 16
				for row in 3:
					buffer[at + 4 * row] = entry.basis.x[row]
					buffer[at + 4 * row + 1] = entry.basis.y[row]
					buffer[at + 4 * row + 2] = entry.basis.z[row]
					buffer[at + 4 * row + 3] = entry.origin[row]
				buffer[at + 12] = color.r
				buffer[at + 13] = color.g
				buffer[at + 14] = color.b
				buffer[at + 15] = color.a
			RenderingServer.multimesh_set_buffer(batch.get_rid(), buffer)
			var instance := MultiMeshInstance3D.new()
			instance.multimesh = batch
			instance.gi_mode = GeometryInstance3D.GI_MODE_STATIC
			if key.x == 4:
				instance.visibility_range_end = 126.0
				instance.visibility_range_end_margin = 18.0
				instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			parent.add_child(instance)


func meshes(key: Vector2i, detail: int) -> Array:
	var coarse := int(detail > 0)
	var id := Vector3i(key.x, key.y, coarse)
	if prototypes.has(id):
		return prototypes[id]
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value ^ (key.x << 24) ^ (key.y << 8) ^ 851835
	var result: Array
	if key.x == 3:
		result = [stone(rng)]
	elif key.x == 4:
		result = [understory(rng, key.y)]
	else:
		result = tree(rng, key.x, coarse != 0)
	prototypes[id] = result
	return result


func start() -> SurfaceTool:
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	return tool


func vertex(tool: SurfaceTool, p: Vector3, normal: Vector3, uv: Vector2, shade: float) -> void:
	tool.set_normal(normal)
	tool.set_uv(uv)
	tool.set_color(Color(shade, shade, shade, 1.0))
	tool.add_vertex(p)


func tube(tool: SurfaceTool, a: Vector3, b: Vector3, r0: float, r1: float, sides: int) -> void:
	var direction := (b - a).normalized()
	var u := direction.cross(Vector3.UP if absf(direction.y) < 0.96 else Vector3.RIGHT).normalized()
	var v := direction.cross(u).normalized()
	var length := a.distance_to(b)
	for j in sides:
		var t0 := float(j) / sides
		var t1 := float(j + 1) / sides
		var n0 := u * cos(t0 * TAU) + v * sin(t0 * TAU)
		var n1 := u * cos(t1 * TAU) + v * sin(t1 * TAU)
		var n0_taper := (n0 + direction * ((r0 - r1) / maxf(0.001, length))).normalized()
		var n1_taper := (n1 + direction * ((r0 - r1) / maxf(0.001, length))).normalized()
		var va := Vector2(t0 * 3.0, a.y / 1.4)
		var vb := Vector2(t1 * 3.0, a.y / 1.4)
		var vc := Vector2(t0 * 3.0, (a.y + length) / 1.4)
		var vd := Vector2(t1 * 3.0, (a.y + length) / 1.4)
		vertex(tool, a + n0 * r0, n0_taper, va, 1.0)
		vertex(tool, b + n0 * r1, n0_taper, vc, 1.0)
		vertex(tool, a + n1 * r0, n1_taper, vb, 1.0)
		vertex(tool, a + n1 * r0, n1_taper, vb, 1.0)
		vertex(tool, b + n0 * r1, n0_taper, vc, 1.0)
		vertex(tool, b + n1 * r1, n1_taper, vd, 1.0)


func spray(tool: SurfaceTool, center: Vector3, u: Vector3, v: Vector3, light: float) -> void:
	var normal := u.cross(v).normalized().lerp(Vector3.UP, 0.34).normalized()
	var p := [center - u - v, center + u - v, center + u + v, center - u + v]
	var uv := [Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(0, 1)]
	for index: int in [0, 2, 1, 0, 3, 2]:
		vertex(tool, p[index], normal, uv[index], light)


func tree(rng: RandomNumberGenerator, species: int, coarse: bool) -> Array:
	var woody := start()
	var foliage := start()
	var pine := species == 1
	var birch := species == 6
	var palm := species == 2
	var acacia := species == 7
	var height := rng.randf_range(12.9, 17.6) if pine else rng.randf_range(8.2, 12.7)
	if palm:
		height = rng.randf_range(8.8, 12.8)
	var lean := Vector3(rng.randf_range(-1.0, 1.0), 0.0, rng.randf_range(-1.0, 1.0))
	var stem := func(t: float) -> Vector3: return Vector3.UP * height * t + lean * t * t
	var width := rng.randf_range(0.21, 0.43) * (0.73 if birch else 1.0)
	for i in 7:
		var a := float(i) / 7.0
		var b := float(i + 1) / 7.0
		var bottom: Vector3 = stem.call(a)
		if i == 0:
			bottom.y -= 0.57
		tube(woody, bottom, stem.call(b), width * pow(1.02 - a, 1.17) * (1.45 if i == 0 else 1.0),
			maxf(0.017, width * pow(1.02 - b, 1.17)), 7 if coarse else 10)
	var wards := (23 if coarse else 43) if pine else (11 if coarse else 18)
	if palm:
		wards = 12
	for i in wards:
		var elevation := (float(i) + rng.randf_range(-0.68, 0.68)) / wards
		var t := clampf(lerpf(0.20 if pine else 0.42, 0.98, elevation), 0.1, 0.98)
		if palm:
			t = 0.965
		var angle := i * 2.39996 + rng.randf_range(-0.46, 0.46)
		var direction := Vector3(cos(angle), 0.0, sin(angle))
		var side := Vector3(-direction.z, 0.0, direction.x)
		var root: Vector3 = stem.call(t)
		var reach := (0.28 + pow(1.0 - elevation, 0.76) * 3.43) * rng.randf_range(0.69, 1.36)
		if not pine:
			reach = (1.15 + 2.7 * sin(elevation * PI + 0.25)) * rng.randf_range(0.74, 1.24)
			reach *= 1.24 if acacia else 0.82 if birch else 1.0
		if palm:
			reach = rng.randf_range(2.6, 4.0)
		var elbow := root + direction * reach * 0.57 + Vector3.UP * (0.55 if pine else 1.0)
		var end := root + direction * reach + Vector3.UP * (-reach * 0.23 + 0.26 if pine else rng.randf_range(0.83, 1.82))
		tube(woody, root, elbow, maxf(0.016, width * (1.0 - t) * 0.64), 0.03, 4)
		tube(woody, elbow, end, 0.03, 0.009, 4)
		for j in (3 if coarse else 6):
			var along := (float(j) + 1.0) / (4.0 if coarse else 7.0)
			var sign_x := 1.0 if j % 2 == 0 else -1.0
			var p := elbow.lerp(end, along) + side * sign_x * rng.randf_range(0.10, 0.75)
			var size := lerpf(0.65, 1.24, 1.0 - t) * rng.randf_range(0.72, 1.18)
			if palm:
				size *= 0.88
			var tilt := side.rotated(Vector3.UP, rng.randf_range(-0.95, 0.95))
			spray(foliage, p, tilt * size,
				(direction * 0.73 + Vector3.UP * rng.randf_range(0.35, 0.72)) * size, 0.55 + 0.35 * along)
			if not coarse:
				spray(foliage, p, (tilt + direction * 0.32) * size * 0.74,
					(Vector3.UP * 0.82 - direction * 0.24) * size, 0.68 + 0.32 * along)
	var stem_mesh := woody.commit()
	var crown_mesh := foliage.commit()
	stem_mesh.surface_set_material(0, material_library.trunks[int(birch)])
	crown_mesh.surface_set_material(0, material_library.crowns[TREES.find(species)])
	return [stem_mesh, crown_mesh]


func understory(rng: RandomNumberGenerator, variant: int) -> Mesh:
	var geometry := start()
	var tall := [0.32, 0.66, 0.48, 0.24][variant] as float
	for blade in 14:
		var a := blade * 2.39996 + rng.randf_range(-0.26, 0.26)
		var direction := Vector3(cos(a), 0.0, sin(a))
		var side := Vector3(-direction.z, 0.0, direction.x)
		var base := direction * rng.randf_range(0.02, 0.23) - Vector3.UP * 0.075
		var length := tall * rng.randf_range(0.62, 1.41)
		var half := rng.randf_range(0.009, 0.037)
		var elbow := base + Vector3.UP * length * 0.61 + direction * length * 0.12
		var tip := base + Vector3.UP * length + direction * length * 0.45
		var normal := (direction * 0.38 + Vector3.UP * 0.78).normalized()
		var points := [base - side * half, base + side * half, elbow - side * half * 0.64,
			elbow + side * half * 0.64, tip]
		var levels := [0.13, 0.13, 0.61, 0.61, 1.0]
		for j: int in [0, 1, 2, 1, 3, 2, 2, 3, 4]:
			vertex(geometry, points[j], normal, Vector2(float(j % 2), levels[j]), 0.49 + levels[j] * 0.47)
	var mesh := geometry.commit()
	mesh.surface_set_material(0, material_library.grass)
	return mesh


func stone(rng: RandomNumberGenerator) -> Mesh:
	var base := SphereMesh.new()
	base.radial_segments = 13
	base.rings = 6
	base.radius = 0.69
	base.height = 1.02
	var arrays := base.get_mesh_arrays()
	var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var colors := PackedColorArray()
	colors.resize(positions.size())
	var phase := rng.randf() * TAU
	for i in positions.size():
		var p := positions[i]
		var n := 1.0 + sin(p.x * 13.0 + p.y * 6.1 + phase) * 0.13
		n += cos(p.z * 17.0 - p.x * 9.0 + phase) * 0.09
		p *= n
		p.y -= 0.15
		p.x *= 1.35
		positions[i] = p
		colors[i] = Color(0.27, 0.25, 0.21, 1.0)
	arrays[Mesh.ARRAY_VERTEX] = positions
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.surface_set_material(0, material_library.rock)
	return mesh
