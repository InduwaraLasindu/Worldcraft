$ErrorActionPreference = 'Stop'
$packageRoot = Join-Path $PWD 'dist/Worldcraft-Windows-x64-development'
New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
Copy-Item -LiteralPath 'target/ci/worldcraft.exe' -Destination (Join-Path $packageRoot 'Worldcraft.exe')
foreach ($name in @('README.md','LICENSE','THIRD_PARTY_NOTICES.md','BUILDING.md','ROADMAP.md')) {
    Copy-Item -LiteralPath $name -Destination $packageRoot
}
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

This is an engineering review build, not a stable release or installer.
Hosted core/desktop tests and the Linux software-rendered smoke check precede packaging.
Interactive behavior and graphics performance on Windows gaming hardware remain unverified.
No Python, Java, AI model or paid API is required at runtime.
Dependency metadata and upstream license/notice files supplied in the resolved crates are included.
"@ | Set-Content -LiteralPath (Join-Path $packageRoot 'BUILD-INFO.txt') -Encoding utf8
$archive = Join-Path $PWD 'dist/Worldcraft-Windows-x64-development.zip'
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archive -Force
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  Worldcraft-Windows-x64-development.zip" |
    Set-Content -LiteralPath 'dist/SHA256SUMS.txt' -Encoding ascii
Write-Output "Packaged $archive ($((Get-Item -LiteralPath $archive).Length) bytes)"
