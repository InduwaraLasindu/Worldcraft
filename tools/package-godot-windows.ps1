$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $project 'dist'
$package = Join-Path $dist 'package'
$version = (Select-String -LiteralPath (Join-Path $project 'game/project.godot') -Pattern '^config/version="([^"]+)"$').Matches[0].Groups[1].Value
$testRoot = Join-Path $env:RUNNER_TEMP 'worldcraft-godot'

function Test-WorldcraftPackage($gameDirectory, $name, [switch]$Regional) {
    $log = Join-Path $dist "$name.log"
    $auditArgs = @('--headless', '--log-file', "`"$log`"", '--', '--audit')
    if ($Regional) {
        $auditArgs += @('--regional', '--seed', '42', '--preset', '0')
    } else {
        $auditArgs += '--waterfall'
    }
    $process = Start-Process -FilePath (Join-Path $gameDirectory 'Worldcraft.exe') `
        -WorkingDirectory $gameDirectory -PassThru `
        -ArgumentList $auditArgs
    if (-not $process.WaitForExit(60000)) {
        $process.Kill()
        throw "The $name game timed out"
    }
    $output = Get-Content -LiteralPath $log -Raw
    Write-Output $output
    if ($process.ExitCode -ne 0 -or $output -match '(?m)SCRIPT ERROR|SHADER ERROR|^ERROR:') {
        throw "The $name game reported a startup error"
    }
    $audit = [regex]::Match($output, '(?m)^WORLDCRAFT_AUDIT (\{.+\})\s*$')
    if (-not $audit.Success) { throw "The $name game produced no scene audit" }
    $scene = $audit.Groups[1].Value | ConvertFrom-Json
    if ($Regional) {
        if ($scene.generator_version -ne 6 -or $scene.terrain_tiles -lt 9 -or -not $scene.horizon) {
            throw "The $name game did not stream the regional terrain foundation"
        }
        return
    }
    if ($scene.terrain_tiles -lt 9 -or -not $scene.horizon -or -not $scene.river -or -not $scene.spray) {
        throw "The $name game produced an incomplete River Valley"
    }
    if ($scene.scenery_objects -lt 10000 -or $scene.scenery_kinds -lt 10) {
        throw "The $name game did not stream the full scenery population"
    }
}

$testZip = Join-Path $dist "Worldcraft-$env:GITHUB_REF_NAME-windows.zip"
$portable = Join-Path $testRoot 'portable'
Expand-Archive -LiteralPath $testZip -DestinationPath $portable
Test-WorldcraftPackage $portable 'portable'

$compiler = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'
& $compiler "/DSourceDir=$package" "/DOutputDir=$dist" "/DProjectRoot=$project" `
    "/DAppVersion=$version" "/DVersionNumbers=$($version.Split('-')[0]).0" (Join-Path $PSScriptRoot 'installer.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }

$installer = Join-Path $dist "Worldcraft-Setup-$version.exe"
$installed = Join-Path $testRoot 'installed'
$process = Start-Process -FilePath $installer -PassThru `
    -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$installed`"")
if (-not $process.WaitForExit(60000)) {
    $process.Kill()
    throw 'The Windows installer timed out'
}
if ($process.ExitCode -ne 0) { throw 'The Windows installer failed' }
Test-WorldcraftPackage $installed 'installed'
Test-WorldcraftPackage $installed 'installed-regional' -Regional
