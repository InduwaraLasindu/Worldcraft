param([Parameter(Mandatory=$true)][string]$Installer,[Parameter(Mandatory=$true)][string]$PackageRoot)
$ErrorActionPreference='Stop'
$testRoot = [IO.Path]::GetFullPath((Join-Path $env:RUNNER_TEMP 'worldcraft-install-audit'))
$permittedRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
if (-not $testRoot.StartsWith($permittedRoot,[StringComparison]::OrdinalIgnoreCase)) { throw 'Installation audit escaped the hosted temporary directory' }
$originalRuntime = $env:WORLDCRAFT_ORT_DYLIB
try {
    Remove-Item Env:WORLDCRAFT_ORT_DYLIB -ErrorAction SilentlyContinue
    $portableReport=Join-Path $env:RUNNER_TEMP 'worldcraft-portable-report.json'
    $process=Start-Process -FilePath (Join-Path $PackageRoot 'Worldcraft.exe') -ArgumentList @('--validate-install',"`"$portableReport`"") -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw 'Portable installation validation failed' }
    $report=Get-Content -LiteralPath $portableReport -Raw | ConvertFrom-Json
    if ($report.status -ne 'passed') { throw 'Portable validation did not pass' }
    $process=Start-Process -FilePath $Installer -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/DIR=`"$testRoot`"") -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw 'Hosted per-user installation failed' }
    $installedReport=Join-Path $env:RUNNER_TEMP 'worldcraft-installed-report.json'
    $process=Start-Process -FilePath (Join-Path $testRoot 'Worldcraft.exe') -ArgumentList @('--validate-install',"`"$installedReport`"") -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw 'Installed application validation failed' }
    $report=Get-Content -LiteralPath $installedReport -Raw | ConvertFrom-Json
    if ($report.status -ne 'passed') { throw 'Installed validation did not pass' }
    $keepFile=Join-Path $testRoot 'preserve-user-file.txt'
    'Uninstall must preserve untracked user files.' | Set-Content -LiteralPath $keepFile
    $uninstaller=Join-Path $testRoot 'unins000.exe'
    $process=Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw 'Hosted uninstall failed' }
    if (Test-Path -LiteralPath (Join-Path $testRoot 'Worldcraft.exe')) { throw 'Uninstall left the executable behind' }
    if (-not (Test-Path -LiteralPath $keepFile)) { throw 'Uninstall removed an untracked user file' }
    Write-Output 'Portable runtime, installed runtime and safe uninstall passed on hosted Windows.'
} finally {
    if ($originalRuntime) { $env:WORLDCRAFT_ORT_DYLIB=$originalRuntime }
}
