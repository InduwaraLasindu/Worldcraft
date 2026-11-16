$ErrorActionPreference='Stop'
$env:WGPU_BACKEND='dx12'
$env:WGPU_DX12_COMPILER='dxc'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$out=Join-Path $PWD 'calibration-output.log'
$err=Join-Path $PWD 'calibration-errors.log'
Remove-Item -LiteralPath 'calibration-audit.json' -ErrorAction SilentlyContinue
$binary=Join-Path $PWD 'target/release/worldcraft.exe'
$process=Start-Process -FilePath $binary -ArgumentList '--calibration-audit' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
try {
    if (-not $process.WaitForExit(180000)) {$process.Kill();$process.WaitForExit();throw 'Windows launch calibration timed out'}
    if ($process.ExitCode -ne 0) {throw 'Windows launch calibration returned an error'}
    $report=Get-Content -LiteralPath 'calibration-audit.json' -Raw | ConvertFrom-Json
    if ($report.status -ne 'passed' -or -not $report.frustum_inside_visible -or -not $report.frustum_behind_culled) {throw 'Launch calibration/culling assertions did not pass'}
    if ($report.automatic_tier -ne 0) {throw 'Software adapter did not select the minimum automatic tier'}
    Write-Output ($report | ConvertTo-Json -Depth 5)
} finally {
    Get-Content -LiteralPath $out -Tail 60 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 60 -ErrorAction SilentlyContinue
}
