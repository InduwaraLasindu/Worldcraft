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


func attach(parent: Node3D, packets: Array, detail: int) -> Vector3i:
	var totals := Vector3i.ZERO
	for packet: Dictionary in packets:
		var species: int = packet.species
		var key := Vector2i(species, packet.variant)
		var buffer: PackedFloat32Array = packet.buffer
		var count := buffer.size() >> 4
		var large := species in TREES || species in [3, 14, 17]
		totals.x += count
		totals.z |= 1 << species
		for prototype: Mesh in meshes(key, detail):
			var batch := MultiMesh.new()
			batch.transform_format = MultiMesh.TRANSFORM_3D
			batch.use_custom_data = true
			batch.mesh = prototype
			batch.instance_count = count
			RenderingServer.multimesh_set_buffer(batch.get_rid(), buffer)
			var instance := MultiMeshInstance3D.new()
			instance.multimesh = batch
			if large:
				instance.gi_mode = GeometryInstance3D.GI_MODE_STATIC
			else:
				instance.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
				instance.visibility_range_end = 165.0 if detail == 0 else 265.0
				instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			parent.add_child(instance)
			totals.y += 1
	return totals

func meshes(key: Vector2i, detail: int) -> Array:
	var coarse := int(detail == 1)
	var id := Vector3i(key.x, key.y, coarse)
	if prototypes.has(id):
		return prototypes[id]
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value ^ (key.x << 24) ^ (key.y << 8) ^ 851835
	var result: Array
	match key.x:
		3, 11, 15:
			result = [stone(rng, key.x)]
		9:
			result = [flower(rng, coarse != 0)]
		12, 14, 17:
			result = [fallen_wood(rng, key.x)]
		_:
			if key.x in TREES:
				result = tree(rng, key.x, coarse != 0)
			else:
				result = [understory(rng, key.x, coarse != 0)]
	prototypes[id] = result
	return result


func start() -> SurfaceTool:
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	return tool


func vertex(tool: SurfaceTool, p: Vector3, normal: Vector3, uv: Vector2, shade: float, marker := 1.0) -> void:
	tool.set_normal(normal)
	tool.set_uv(uv)
	tool.set_color(Color(shade, shade, shade, marker))
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
		var elbow := root + direction * reach * 0.57 + Vector3.UP * (reach * -0.17 if pine else 1.0)
		var end := root + direction * reach + Vector3.UP * (reach * rng.randf_range(-0.06, 0.21) if pine else rng.randf_range(0.52, 1.88))
		tube(woody, root, elbow, maxf(0.016, width * (1.0 - t) * 0.64), 0.03, 4)
		tube(woody, elbow, end, 0.03, 0.009, 4)
		for j in (3 if coarse else 6):
			var along := (float(j) + 1.0) / (4.0 if coarse else 7.0)
			var sign_x := 1.0 if j % 2 == 0 else -1.0
			var p := root.lerp(elbow, along * 1.65) if along < 0.6 else elbow.lerp(end, (along - 0.6) * 2.5)
			p += side * sign_x * rng.randf_range(0.11, 0.65) * (1.0 - along * 0.44)
			var size := lerpf(0.54, 1.38, 1.0 - t) * rng.randf_range(0.72, 1.24)
			if palm:
				size *= 1.16
			var tilt := side.rotated(Vector3.UP, rng.randf_range(-0.95, 0.95))
			spray(foliage, p, tilt * size,
				(direction * 0.73 + Vector3.UP * rng.randf_range(-0.1, 0.64)) * size, 0.55 + 0.35 * along)
			if not coarse:
				spray(foliage, p, (tilt + direction * 0.32) * size * 0.74,
					(Vector3.UP * 0.82 - direction * 0.24) * size, 0.68 + 0.32 * along)
	if not coarse && not palm:
		for i in 4:
			var angle := i * TAU / 4.0 + rng.randf_range(-0.47, 0.47)
			var root_dir := Vector3(cos(angle), 0.0, sin(angle))
			var base := root_dir * width * 0.49 + Vector3.UP * rng.randf_range(0.23, 0.52)
			var knot := root_dir * width * rng.randf_range(1.5, 2.2)
			var tip := root_dir * width * rng.randf_range(3.2, 4.8) - Vector3.UP * 0.32
			tube(woody, base, knot, width * 0.55, width * 0.24, 5)
			tube(woody, knot, tip, width * 0.24, width * 0.05, 5)
	var stem_mesh := woody.commit()
	var crown_mesh := foliage.commit()
	stem_mesh.surface_set_material(0, material_library.trunks[int(birch)])
	crown_mesh.surface_set_material(0, material_library.crowns[TREES.find(species)])
	return [stem_mesh, crown_mesh]


func understory(rng: RandomNumberGenerator, species: int, coarse: bool) -> Mesh:
	var geometry := start()
	if species == 5 || species == 8:
		for i in (4 if coarse else 7 if species == 8 else 10):
			var a := i * 2.39996 + rng.randf_range(-0.22, 0.22)
			var direction := Vector3(cos(a), 0.0, sin(a))
			var side := Vector3(-direction.z, 0.0, direction.x)
			var size := rng.randf_range(0.44, 0.79)
			var height := rng.randf_range(0.22, 0.65) if species == 8 else rng.randf_range(0.49, 0.99)
			var tip := direction * size + Vector3.UP * height
			spray(geometry, tip * 0.52, side * size * 0.53, tip * 0.54, rng.randf_range(0.61, 0.96))
		var foliage := geometry.commit()
		foliage.surface_set_material(0, material_library.fern if species == 8 else material_library.crowns[0])
		return foliage
	var tall := 1.04 if species == 10 else 0.67 if species == 16 else 0.44
	for blade in (5 if coarse else 9 if species == 16 else 11):
		var a := blade * 2.39996 + rng.randf_range(-0.26, 0.26)
		var direction := Vector3(cos(a), 0.0, sin(a))
		var side := Vector3(-direction.z, 0.0, direction.x)
		var base := direction * rng.randf_range(0.02, 0.23) - Vector3.UP * 0.075
		var length := tall * rng.randf_range(0.62, 1.41)
		var half := rng.randf_range(0.011, 0.034)
		if species == 16:
			half *= 4.4
		var elbow := base + Vector3.UP * length * 0.61 + direction * length * 0.12
		var tip := base + Vector3.UP * length + direction * length * 0.45
		var normal := (direction * 0.38 + Vector3.UP * 0.78).normalized()
		var points := [base - side * half, base + side * half, elbow - side * half * 0.64,
			elbow + side * half * 0.64, tip]
		var levels := [0.13, 0.13, 0.61, 0.61, 1.0]
		for j: int in [0, 1, 2, 1, 3, 2, 2, 3, 4]:
			vertex(geometry, points[j], normal, Vector2(float(j % 2), levels[j]), 0.49 + levels[j] * 0.47)
	var mesh := geometry.commit()
	mesh.surface_set_material(0, material_library.dry_grass if species == 13 else material_library.succulent if species == 16 else material_library.grass)
	return mesh


func stone(rng: RandomNumberGenerator, species: int) -> Mesh:
	var moss := species == 15
	var large := species == 3
	var base := SphereMesh.new()
	base.radial_segments = 13 if large else 7
	base.rings = 6 if large else 2
	base.radius = 0.69 if large else 0.64 if moss else 0.33
	base.height = 1.02 if large else 0.19 if moss else 0.36
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
		p.y -= 0.15 if large else 0.028 if moss else 0.05
		p.x *= 1.35
		positions[i] = p
		colors[i] = Color(0.095, 0.17, 0.041, 0.0) if moss else Color(0.27, 0.25, 0.21, 1.0)
	arrays[Mesh.ARRAY_VERTEX] = positions
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.surface_set_material(0, material_library.moss if moss else material_library.rock)
	return mesh


func flower(rng: RandomNumberGenerator, coarse: bool) -> Mesh:
	var geometry := start()
	for stem in (1 if coarse else 3):
		var a := stem * 2.39996 + rng.randf_range(-0.23, 0.23)
		var spread := Vector3(cos(a), 0.0, sin(a))
		var tip := spread * rng.randf_range(0.04, 0.21) + Vector3.UP * rng.randf_range(0.29, 0.65)
		tube(geometry, spread * 0.055, tip, 0.018, 0.008, 3)
		var size := rng.randf_range(0.058, 0.098)
		var up := Vector3.UP.lerp(spread, 0.36).normalized()
		var tangent := up.cross(spread).normalized()
		var bitangent := tangent.cross(up)
		for petal in (4 if coarse else 6):
			var angle := TAU * petal / (4.0 if coarse else 6.0)
			var out := tangent * cos(angle) + bitangent * sin(angle)
			var side := up.cross(out)
			var center := tip + out * size * 0.70
			vertex(geometry, tip + up * 0.006, up, Vector2(0.5, 0.0), 0.88, 0.3)
			vertex(geometry, center + side * size * 0.38, up, Vector2(0, 0.5), 0.92, 0.0)
			vertex(geometry, tip + out * size * 1.35 + up * 0.025, up, Vector2(0.5, 1), 1.0, 0.0)
			vertex(geometry, tip + out * size * 1.35 + up * 0.025, up, Vector2(0.5, 1), 1.0, 0.0)
			vertex(geometry, center - side * size * 0.38, up, Vector2(1, 0.5), 0.92, 0.0)
			vertex(geometry, tip + up * 0.006, up, Vector2(0.5, 0.0), 0.88, 0.3)
	var mesh := geometry.commit()
	mesh.surface_set_material(0, material_library.flowers)
	return mesh


func fallen_wood(rng: RandomNumberGenerator, species: int) -> Mesh:
	var geometry := start()
	var litter := species == 12
	var radius := rng.randf_range(0.030, 0.069) if litter else rng.randf_range(0.17, 0.31)
	var length := rng.randf_range(0.50, 0.99) if litter else rng.randf_range(2.30, 3.60)
	var sides := 4 if litter else 7
	var start_point := Vector3(0.0, radius * 0.53, -length * 0.48)
	var bend := Vector3(rng.randf_range(-0.12, 0.12) * length, radius * 0.78, length * 0.10)
	var end := Vector3(rng.randf_range(-0.18, 0.18) * length, radius * 0.54, length * 0.53)
	tube(geometry, start_point, bend, radius, radius * 0.81, sides)
	tube(geometry, bend, end, radius * 0.81, radius * 0.36, sides)
	for j in (1 if litter else 3):
		var point := start_point.lerp(end, rng.randf_range(0.22, 0.86))
		var dir := -1.0 if j % 2 == 0 else 1.0
		var broken := point + Vector3(dir * radius * 3.3, radius * 1.7, radius * 2.16)
		tube(geometry, point, broken, radius * 0.32, radius * 0.085, 4)
	if not litter:
		cap(geometry, start_point, (start_point - bend).normalized(), radius, sides)
		cap(geometry, end, (end - bend).normalized(), radius * 0.36, sides)
	var mesh := geometry.commit()
	mesh.surface_set_material(0, material_library.trunks[1 if species == 17 else 0])
	return mesh


func cap(tool: SurfaceTool, point: Vector3, axis: Vector3, radius: float, sides: int) -> void:
	var u := axis.cross(Vector3.UP).normalized()
	var v := axis.cross(u)
	for i in sides:
		vertex(tool, point, axis, Vector2(0.5, 0.5), 1.0, 0.0)
		for j in [i + 1, i]:
			var angle := float(j) * TAU / sides
			var s := Vector2(cos(angle), sin(angle))
			vertex(tool, point + (u * s.x + v * s.y) * radius, axis, s * 0.5 + Vector2(0.5, 0.5), 1.0, 0.0)
