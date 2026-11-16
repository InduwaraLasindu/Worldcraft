$ErrorActionPreference='Stop'
./tools/validate-package-scripts.ps1
python tools/ci/bootstrap_dxc.py
if ($LASTEXITCODE -ne 0) { throw 'DXC preparation failed' }
$env:WGPU_DX12_COMPILER='dxc'
$env:WGPU_BACKEND='dx12'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$out=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native.log'
$err=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native-errors.log'
$binary=Join-Path $PWD 'target/release/worldcraft.exe'
foreach ($name in @('smoke','balanced','rebased','high','home','alpine','coast','map')) {
    $capture=Join-Path $PWD "worldcraft-$name.png"
    if (Test-Path -LiteralPath $capture) { Remove-Item -LiteralPath $capture }
}
$process=Start-Process -FilePath $binary -ArgumentList '--smoke-test' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
$deadline=[DateTime]::UtcNow.AddMinutes(25)
try {
    while (-not $process.WaitForExit(15000)) {
        $process.Refresh()
        Write-Output "Hosted Windows graphics running at $([DateTime]::UtcNow.ToString('o')); CPU seconds=$($process.TotalProcessorTime.TotalSeconds); resident MiB=$([math]::Round($process.WorkingSet64/1MB))"
        Get-Content -LiteralPath $out -Tail 4 -ErrorAction SilentlyContinue
        Get-Content -LiteralPath $err -Tail 4 -ErrorAction SilentlyContinue
        if ([DateTime]::UtcNow -gt $deadline) { $process.Kill();$process.WaitForExit();throw 'Hosted Windows native integration exceeded 25 minutes' }
    }
    if ($process.ExitCode -ne 0) { throw "Native Windows integration failed with exit code $($process.ExitCode)" }
    $logs=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join "`n"
    if ($logs -notmatch 'GRAPHICS:.*Microsoft Basic Render Driver.*Dx12') { throw 'Expected hosted software Direct3D 12 adapter was not observed' }
    if ($logs -match 'error validating shader|ShaderValidationError|panicked at') { throw 'Native Windows shader/runtime validation failed' }
    Add-Type -AssemblyName System.Drawing
    $imageAudit = @()
    foreach ($name in @('smoke','balanced','rebased','high','home','alpine','coast','map')) {
        $path="worldcraft-$name.png"
        if (-not (Test-Path -LiteralPath $path)) {throw "Missing Windows native capture $name"}
        $image=[System.Drawing.Bitmap]::FromFile((Join-Path $PWD $path))
        try {
            if ($image.Width -ne 400 -or $image.Height -ne 300) { throw "Unexpected Windows capture dimensions: $name" }
            $values=[System.Collections.Generic.List[double]]::new()
            for ($y=5;$y -lt $image.Height;$y+=10) {
                for ($x=5;$x -lt $image.Width;$x+=10) {
                    $pixel=$image.GetPixel($x,$y)
                    $values.Add(($pixel.R+$pixel.G+$pixel.B)/3.0)
                }
            }
            $stats=$values | Measure-Object -Average -Minimum -Maximum
            if (($stats.Maximum-$stats.Minimum) -lt 8) { throw "Windows native capture lacks visible variation: $name" }
            $imageAudit += @{scene=$name;width=$image.Width;height=$image.Height;brightness_range=($stats.Maximum-$stats.Minimum)}
        } finally { $image.Dispose() }
    }
    $imageAudit | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath 'windows-render-image-audit.json' -Encoding utf8
    python tools/ci/publish_windows_audit.py
    if ($LASTEXITCODE -ne 0) { throw 'Windows render evidence could not be recorded' }
    Write-Output 'Native Windows integration passed at a 400x300 software QA viewport. This is not a gaming performance test.'
} finally {
    Get-Content -LiteralPath $out -Tail 100 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 100 -ErrorAction SilentlyContinue
}
