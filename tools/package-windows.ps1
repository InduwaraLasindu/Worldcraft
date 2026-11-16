$ErrorActionPreference = 'Stop'
$packageRoot = Join-Path $PWD 'dist/Worldcraft-Windows-x64-development'
New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
Copy-Item -LiteralPath 'target/release/worldcraft.exe' -Destination (Join-Path $packageRoot 'Worldcraft.exe')
foreach ($name in @('README.md','LICENSE','THIRD_PARTY_NOTICES.md','BUILDING.md','ROADMAP.md')) {
    Copy-Item -LiteralPath $name -Destination $packageRoot
}
Copy-Item -LiteralPath 'licenses' -Destination $packageRoot -Recurse
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
$metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
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
Hosted core/desktop tests and the Linux software-rendered smoke check precede packaging.
Interactive behavior and graphics performance on Windows gaming hardware remain unverified.
No Python, Java or paid API is required. Classical generation and the baked AI example work offline.
New AI regions use the optional checksum-pinned 22.5 MB Terrain Diffusion coarse model.
Native inference uses ONNX Runtime CPU; full latent/decoder generation and GPU inference are not claimed.
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
