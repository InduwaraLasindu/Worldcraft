param([ValidateSet('release','ci')][string]$Profile='release')
$ErrorActionPreference='Stop'
$env:WGPU_BACKEND='dx12'
$env:WGPU_DX12_COMPILER='dxc'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$out=Join-Path $env:RUNNER_TEMP 'worldcraft-render-settings.log'
$err=Join-Path $env:RUNNER_TEMP 'worldcraft-render-settings-errors.log'
Get-ChildItem -Path 'worldcraft-render-*.png' -ErrorAction SilentlyContinue | Remove-Item
Remove-Item -LiteralPath 'renderer-settings-audit.json' -ErrorAction SilentlyContinue
$process=Start-Process -FilePath (Join-Path $PWD "target/$Profile/worldcraft.exe") -ArgumentList '--render-smoke-test' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
$deadline=[DateTime]::UtcNow.AddMinutes(12)
$peak=0L
$hardErrors='failed to create (shader module|render pipeline|compute pipeline)|validation error|failed to process shader|failed to resolve shader|error validating shader|ShaderValidationError|panicked at|Caught rendering error|OutOfMemory RenderError|Failed to send screenshot'
try {
    while (-not $process.WaitForExit(10000)) {
        $process.Refresh()
        $peak=[Math]::Max($peak,$process.WorkingSet64)
        $logs=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join "`n"
        if ($logs -match $hardErrors -or $peak -gt 6GB -or [DateTime]::UtcNow -gt $deadline) {
            $process.Kill();$process.WaitForExit();throw 'Render-settings audit failed its renderer, memory or timeout gate'
        }
        Write-Output "Render-settings native route running; peak MiB=$([Math]::Round($peak/1MB))"
    }
    if ($process.ExitCode -ne 0) {throw "Render-settings route exited $($process.ExitCode)"}
    $logs=((Get-Content -LiteralPath $out -Raw),(Get-Content -LiteralPath $err -Raw)) -join "`n"
    if ($logs -match $hardErrors -or $logs -notmatch 'RENDER AUDIT:') {throw 'Render-settings shader/state gate failed'}
    python tools/ci/check_render_images.py
    if ($LASTEXITCODE -ne 0) {throw 'Render-settings pixel/ownership gate failed'}
} finally {
    Copy-Item -LiteralPath $out -Destination 'native-render-settings.log' -ErrorAction SilentlyContinue
    Copy-Item -LiteralPath $err -Destination 'native-render-settings-errors.log' -ErrorAction SilentlyContinue
    @{source=$env:GITHUB_SHA;peak_resident_bytes=$peak;scope='Hosted software DX12; unsupported vendor dispatches are skips'} |
        ConvertTo-Json | Set-Content -LiteralPath 'windows-render-settings-resource-audit.json' -Encoding utf8
    Get-Content -LiteralPath $out -Tail 50 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 50 -ErrorAction SilentlyContinue
}
