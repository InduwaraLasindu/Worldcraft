extends RefCounted
## A single seeded, asynchronously baked mip chain shared by the analytic PBR recipes.

const TERRAIN = preload("res://shaders/terrain.gdshader")
const BARK = preload("res://shaders/bark.gdshader")
const LEAVES = preload("res://shaders/leaves.gdshader")
const CANOPY = preload("res://shaders/canopy.gdshader")
const WATER = preload("res://shaders/water.gdshader")
const FOG = preload("res://shaders/fog.gdshader")
const SKY = preload("res://shaders/sky.gdshader")

var terrain: ShaderMaterial
var vista: ShaderMaterial
var rock: ShaderMaterial
var trunks: Array[ShaderMaterial] = []
var tree_bark: Array[ShaderMaterial] = []
var tree_canopy: Array[ShaderMaterial] = []
var shrub: ShaderMaterial
var grass: ShaderMaterial
var dry_grass: ShaderMaterial
var succulent: ShaderMaterial
var flowers: ShaderMaterial
var moss: ShaderMaterial
var fern: ShaderMaterial
var river: ShaderMaterial
var sea: ShaderMaterial
var fog: ShaderMaterial
var plunge: ShaderMaterial
var sky: ShaderMaterial
var maps: Array[ShaderMaterial] = []
var active_side := 64
var active_seed := 0
var wanted := Vector2i.ZERO
var pending: NoiseTexture2D
var pending_key := Vector2i.ZERO
var field: Texture2D
var seed_shift := Vector3.ZERO


func initialize(seed_value: int) -> void:
	seed_shift = shift(seed_value)
	active_seed = noise_seed(seed_value)
	var noise := recipe(active_seed, 64)
	field = ImageTexture.create_from_image(noise.get_seamless_image(64, 64))
	terrain = make(TERRAIN)
	vista = make(TERRAIN, {"surface_type": 2})
	rock = make(TERRAIN, {"surface_type": 1})
	trunks = [make(BARK), make(BARK, {"kind": 1.0})]
	var heights := [12.5, 16.8, 11.6, 13.4, 9.2]
	var palette := [Color(0.39, 0.51, 0.24), Color(0.27, 0.39, 0.22), Color(0.38, 0.55, 0.22),
		Color(0.44, 0.56, 0.24), Color(0.50, 0.53, 0.26)]
	for family in heights.size():
		var shape := {"tree_height": heights[family], "flex": [0.68, 0.51, 1.17, 0.96, 0.74][family]}
		tree_bark.append(make(BARK, shape.merged({"kind": [0.0, 2.0, 3.0, 1.0, 4.0][family]})))
		tree_canopy.append(make(CANOPY, shape.merged({"family": family, "pigment": palette[family]})))
	shrub = make(LEAVES, {"pigment": Color(0.37, 0.49, 0.22)})
	grass = make(LEAVES, {"pigment": Color(0.40, 0.51, 0.28), "blade": true})
	dry_grass = make(LEAVES, {"pigment": Color(0.64, 0.53, 0.29), "blade": true})
	succulent = make(LEAVES, {"pigment": Color(0.32, 0.47, 0.37), "blade": true, "flex": 0.25})
	flowers = make(LEAVES, {"pigment": Color(0.35, 0.48, 0.24), "blade": true, "wildflower": true})
	moss = make(TERRAIN, {"surface_type": 3, "relief": 0.54})
	fern = make(LEAVES, {"pigment": Color(0.31, 0.44, 0.20), "species": 2.0})
	river = make(WATER, {"inland": true})
	sea = make(WATER)
	fog = make(FOG)
	plunge = make(FOG, {"plunge": true})
	sky = make(SKY)


func make(shader_resource: Shader, params := {}) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = shader_resource
	material.set_shader_parameter("ground_field", field)
	material.set_shader_parameter("seed_shift", seed_shift)
	for name in params:
		material.set_shader_parameter(name, params[name])
	maps.append(material)
	return material


func set_tree_atlas(bytes: PackedByteArray) -> void:
	var image := Image.create_from_data(1024, 512, false, Image.FORMAT_RGBA8, bytes)
	image.generate_mipmaps()
	var atlas := ImageTexture.create_from_image(image)
	for material in tree_canopy:
		material.set_shader_parameter("foliage_atlas", atlas)


func shift(value: int) -> Vector3:
	var rng := RandomNumberGenerator.new()
	rng.seed = value
	return Vector3(rng.randf_range(-47.0, 47.0), rng.randf_range(-14.0, 14.0), rng.randf_range(-47.0, 47.0))


func noise_seed(value: int) -> int:
	return int((value ^ (value >> 32)) & 0x7fffffff)


func recipe(value: int, side: int) -> FastNoiseLite:
	var noise := FastNoiseLite.new()
	noise.seed = value
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = 0.024 * 1024.0 / side
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 4
	noise.fractal_gain = 0.52
	return noise


func request(seed_value: int, side: int) -> void:
	seed_shift = shift(seed_value)
	for material in maps:
		material.set_shader_parameter("seed_shift", seed_shift)
	wanted = Vector2i(noise_seed(seed_value), side)
	_start_job()


func _start_job() -> void:
	if pending != null || wanted == Vector2i(active_seed, active_side):
		return
	pending_key = wanted
	var job := NoiseTexture2D.new()
	job.width = wanted.y
	job.height = wanted.y
	job.seamless = true
	job.seamless_blend_skirt = 0.18
	job.generate_mipmaps = true
	job.noise = recipe(wanted.x, wanted.y)
	pending = job
	job.changed.connect(_finish_job, CONNECT_ONE_SHOT | CONNECT_DEFERRED)


func _finish_job() -> void:
	if pending_key == wanted:
		field = pending
		active_seed = wanted.x
		active_side = wanted.y
		for material in maps:
			material.set_shader_parameter("ground_field", field)
	pending = null
	_start_job()


func animate(t: float, wind: Vector2, wet: float, snow: float, dust: float, shift_3d: Vector3) -> void:
	for material: ShaderMaterial in [terrain, rock, vista, moss]:
		material.set_shader_parameter("world_shift", shift_3d)
		material.set_shader_parameter("wetness", wet)
		material.set_shader_parameter("snowfall", snow)
		material.set_shader_parameter("grit", dust)
	for material: ShaderMaterial in trunks:
		material.set_shader_parameter("wetness", wet)
	for material: ShaderMaterial in tree_bark + tree_canopy + [shrub, grass, fern, dry_grass, succulent, flowers]:
		material.set_shader_parameter("clock", t)
		material.set_shader_parameter("wind", wind)
		material.set_shader_parameter("wetness", wet)
		material.set_shader_parameter("snowfall", snow)
	for material: ShaderMaterial in [river, sea]:
		material.set_shader_parameter("world_shift", shift_3d)
		material.set_shader_parameter("clock", t)
		material.set_shader_parameter("wind", wind)
