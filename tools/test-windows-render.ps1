$ErrorActionPreference='Stop'
$env:WGPU_BACKEND='dx12'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$out=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native.log'
$err=Join-Path $env:RUNNER_TEMP 'worldcraft-windows-native-errors.log'
$binary=Join-Path $PWD 'target/release/worldcraft.exe'
$process=Start-Process -FilePath $binary -ArgumentList '--smoke-test' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
$deadline=[DateTime]::UtcNow.AddMinutes(10)
try {
    while (-not $process.WaitForExit(15000)) {
        Write-Output "Hosted Windows graphics running at $([DateTime]::UtcNow.ToString('o'))"
        Get-Content -LiteralPath $out -Tail 4 -ErrorAction SilentlyContinue
        Get-Content -LiteralPath $err -Tail 4 -ErrorAction SilentlyContinue
        if ([DateTime]::UtcNow -gt $deadline) { $process.Kill();$process.WaitForExit();throw 'Hosted Windows native integration exceeded 10 minutes' }
    }
    if ($process.ExitCode -ne 0) { throw "Native Windows integration failed with exit code $($process.ExitCode)" }
    $logs=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join "`n"
    if ($logs -match 'error validating shader|ShaderValidationError|panicked at') { throw 'Native Windows shader/runtime validation failed' }
    foreach ($name in @('smoke','balanced','rebased','high','home','alpine','coast','map')) {
        if (-not (Test-Path -LiteralPath "worldcraft-$name.png")) {throw "Missing Windows native capture $name"}
    }
    Write-Output 'Native Windows integration passed at a 400x300 software QA viewport. This is not a gaming performance test.'
} finally {
    Get-Content -LiteralPath $out -Tail 100 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 100 -ErrorAction SilentlyContinue
}
