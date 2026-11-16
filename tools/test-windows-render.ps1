$ErrorActionPreference='Stop'
$env:WGPU_BACKEND='dx12'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$out=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native.log'
$err=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native-errors.log'
$binary=Join-Path $PWD 'target/ci/worldcraft.exe'
$process=Start-Process -FilePath $binary -ArgumentList '--smoke-test' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
if (-not $process.WaitForExit(600000)) { $process.Kill(); throw 'Hosted Windows native integration exceeded 10 minutes' }
Get-Content -LiteralPath $out -ErrorAction SilentlyContinue | Select-Object -Last 100
Get-Content -LiteralPath $err -ErrorAction SilentlyContinue | Select-Object -Last 100
if ($process.ExitCode -ne 0) { throw "Native Windows integration failed with exit code $($process.ExitCode)" }
$logs=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join "`n"
if ($logs -match 'error validating shader|ShaderValidationError|panicked at') { throw 'Native Windows shader/runtime validation failed' }
foreach ($name in @('smoke','balanced','rebased','high','home','alpine','coast','map')) {
    if (-not (Test-Path -LiteralPath "worldcraft-$name.png")) {throw "Missing Windows native capture $name"}
}
Write-Output 'Native Windows create/explore/edit/save and shader integration passed using software Direct3D 12.'
