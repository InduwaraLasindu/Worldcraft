$ErrorActionPreference = 'Stop'
$packageRoot = Join-Path $PWD 'dist/Worldcraft-Windows-x64-development'
New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
Copy-Item -LiteralPath 'target/release/worldcraft.exe' -Destination (Join-Path $packageRoot 'Worldcraft.exe')
foreach ($name in @('README.md','LICENSE','THIRD_PARTY_NOTICES.md','BUILDING.md','ROADMAP.md','CHANGELOG.md','MASTER_SPEC_COVERAGE.md','ARCHITECTURE.md','GENERATION.md','RENDERING.md','PERFORMANCE.md')) {
    Copy-Item -LiteralPath $name -Destination $packageRoot
}
$materialNotices = Join-Path $packageRoot 'material-notices'
New-Item -ItemType Directory -Force -Path $materialNotices | Out-Null
Copy-Item -LiteralPath 'tools/material_sources.json' -Destination $materialNotices
Copy-Item -LiteralPath 'crates/worldcraft/embedded/manifest.json' -Destination $materialNotices
Copy-Item -LiteralPath 'licenses' -Destination $packageRoot -Recurse
Copy-Item -LiteralPath 'examples' -Destination $packageRoot -Recurse
if ($env:WORLDCRAFT_UPSCALER_ROOT) {
    python tools/ci/bootstrap_upscalers.py --root $env:WORLDCRAFT_UPSCALER_ROOT --copy-to $packageRoot
    if ($LASTEXITCODE -ne 0) { throw 'Verified upscaler runtime packaging failed' }
    Copy-Item -LiteralPath 'tools/upscaler_sources.json' -Destination $packageRoot
}
foreach ($name in @('dxcompiler.dll','dxil.dll')) {
    if (-not (Test-Path -LiteralPath $name -PathType Leaf)) { throw "Verified DXC component is missing: $name" }
    Copy-Item -LiteralPath $name -Destination $packageRoot
}
Copy-Item -LiteralPath 'dxc-notices' -Destination $packageRoot -Recurse
if (-not $env:WORLDCRAFT_ORT_DYLIB) { throw 'Verified native AI runtime was not prepared' }
$runtimeLib = Split-Path -Parent $env:WORLDCRAFT_ORT_DYLIB
$runtimeRoot = Split-Path -Parent $runtimeLib
$nativeDestination = Join-Path $packageRoot 'lib'
New-Item -ItemType Directory -Force -Path $nativeDestination | Out-Null
Get-ChildItem -LiteralPath $runtimeLib -File -Filter '*.dll' | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $nativeDestination
}
$runtimeNotices = Join-Path $packageRoot 'onnxruntime-notices'
New-Item -ItemType Directory -Force -Path $runtimeNotices | Out-Null
Get-ChildItem -LiteralPath $runtimeRoot -File | Where-Object {
    $_.Name -match 'LICENSE|NOTICE|VERSION|PRIVACY'
} | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $runtimeNotices }
$metadataArgs = @('metadata','--locked','--format-version','1')
if ($env:WORLDCRAFT_UPSCALER_ROOT) { $metadataArgs += @('--features','worldcraft/nvidia-dlss') }
$metadata = & cargo @metadataArgs | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Could not read dependency metadata' }
$licenseRoot = Join-Path $packageRoot 'dependency-licenses'
New-Item -ItemType Directory -Force -Path $licenseRoot | Out-Null
$index = [System.Collections.Generic.List[string]]::new()
foreach ($package in ($metadata.packages | Sort-Object name,version)) {
    $index.Add("$($package.name) $($package.version) — $($package.license)")
    $index.Add("Repository: $($package.repository)")
    $index.Add("Authors: $($package.authors -join ', ')")
    $index.Add('')
    $sourceRoot = Split-Path -Parent $package.manifest_path
    $destination = Join-Path $licenseRoot "$($package.name)-$($package.version)"
    $legalFiles = @(Get-ChildItem -LiteralPath $sourceRoot -File |
        Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)([.-]|$)' })
    if ($package.license_file) {
        $explicitLicense = Join-Path $sourceRoot $package.license_file
        if (Test-Path -LiteralPath $explicitLicense -PathType Leaf) {
            $legalFiles += Get-Item -LiteralPath $explicitLicense
        }
    }
    if ($legalFiles.Count -gt 0) {
        New-Item -ItemType Directory -Force -Path $destination | Out-Null
        foreach ($file in ($legalFiles | Sort-Object FullName -Unique)) {
            Copy-Item -LiteralPath $file.FullName -Destination $destination
        }
    }
}
$index | Set-Content -LiteralPath (Join-Path $packageRoot 'DEPENDENCIES.txt') -Encoding utf8
$sourceCommit = git rev-parse HEAD
$rustVersion = rustc --version
@"
Worldcraft native Windows x64 development build
Source commit: $sourceCommit
Compiler: $rustVersion

This is an alpha engineering build. The complete master specification is not finished.
Executable profile: release (optimized Rust code).
NVIDIA DLSS/DLAA feature compiled: $([bool]$env:WORLDCRAFT_UPSCALER_ROOT)
Upscaler runtimes: pinned NVIDIA 310.9.1 SR/RR and AMD SDK 2.4.0 Vulkan, when enabled.
RR is a Bevy runtime dependency; this application exposes SR and DLAA, not ray reconstruction.
Actual vendor support is queried from the initialized device; software QA skips unsupported vendor execution.
Modern DXC shader compiler v1.9.2609 and official notices are bundled.
Hosted core/desktop tests and the Linux software-rendered smoke check precede packaging.
Interactive behavior and graphics performance on Windows gaming hardware remain unverified.
No Python, Java or paid API is required. Classical generation and the baked AI example work offline.
New AI regions use an optional pinned 22.5 MB fast model or 2.28 GB detailed model set.
Native CPU inference loads the coarse, base and residual models sequentially for finite detailed bakes.
Infinite learned streaming, upstream conditioning parity and GPU inference remain unfinished.
Dependency metadata and upstream license/notice files supplied in the resolved crates are included.
"@ | Set-Content -LiteralPath (Join-Path $packageRoot 'BUILD-INFO.txt') -Encoding utf8
$appVersion = ($metadata.packages | Where-Object name -eq 'worldcraft').version
$compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
$compilerPath = if ($compiler) { $compiler.Source } else { Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe' }
if (-not (Test-Path -LiteralPath $compilerPath -PathType Leaf)) {
    throw 'The standard hosted Windows image is missing Inno Setup'
}
$distRoot = Join-Path $PWD 'dist'
& $compilerPath "/DSourceDir=$packageRoot" "/DOutputDir=$distRoot" "/DProjectRoot=$PWD" "/DAppVersion=$appVersion" "/DVersionNumbers=$($appVersion.Split('-')[0]).0" 'tools/installer.iss'
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
$installer = Join-Path $distRoot "Worldcraft-Setup-$appVersion.exe"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'Installer output is missing' }
./tools/test-windows-package.ps1 -Installer $installer -PackageRoot $packageRoot
$archive = Join-Path $PWD 'dist/Worldcraft-Windows-x64-development.zip'
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archive -Force
$hashes = @($archive,$installer) | ForEach-Object {
    "$((Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($_))"
}
$hashes | Set-Content -LiteralPath 'dist/SHA256SUMS.txt' -Encoding ascii
Write-Output "Packaged portable ZIP and per-user installer for $appVersion"
