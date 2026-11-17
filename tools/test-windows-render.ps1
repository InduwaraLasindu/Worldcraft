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

# Reduce fill/queue pressure on the hosted software adapter. This only resizes
# this test process's window; it never shows or activates another window.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class WorldcraftQaWindow {
    public delegate bool Callback(IntPtr handle, IntPtr data);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder text, int length);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window,IntPtr after,int x,int y,int width,int height,uint flags);
    public static bool Resize(uint process) {
        bool found=false;
        EnumWindows((window,data)=>{
            uint owner;GetWindowThreadProcessId(window,out owner);
            if(owner!=process)return true;
            var title=new StringBuilder(256);GetWindowText(window,title,256);
            if(!title.ToString().StartsWith("Worldcraft"))return true;
            Rect client,outer;GetClientRect(window,out client);GetWindowRect(window,out outer);
            found=SetWindowPos(window,IntPtr.Zero,0,0,128+(outer.Right-outer.Left)-(client.Right-client.Left),96+(outer.Bottom-outer.Top)-(client.Bottom-client.Top),0x0016);
            return false;
        },IntPtr.Zero);
        return found;
    }
}
'@
$resizeDeadline=[DateTime]::UtcNow.AddSeconds(15)
$resized=$false
while (-not $process.HasExited -and [DateTime]::UtcNow -lt $resizeDeadline) {
    if ([WorldcraftQaWindow]::Resize([uint32]$process.Id)) { $resized=$true;break }
    Start-Sleep -Milliseconds 100
}
if (-not $resized) { if (-not $process.HasExited) {$process.Kill()};throw 'Hosted QA window could not be resized' }
Write-Output 'Hosted software viewport resized to 128x96. All scene and state assertions remain enabled.'

$deadline=[DateTime]::UtcNow.AddMinutes(15)
$memorySamples=[System.Collections.Generic.List[object]]::new()
$peakResidentBytes=0L
try {
    while (-not $process.WaitForExit(15000)) {
        $process.Refresh()
        $peakResidentBytes=[math]::Max($peakResidentBytes,$process.WorkingSet64)
        $memorySamples.Add(@{utc=[DateTime]::UtcNow.ToString('o');resident_bytes=$process.WorkingSet64})
        Write-Output "Hosted Windows graphics running at $([DateTime]::UtcNow.ToString('o')); CPU seconds=$($process.TotalProcessorTime.TotalSeconds); resident MiB=$([math]::Round($process.WorkingSet64/1MB))"
        Get-Content -LiteralPath $out -Tail 8 -ErrorAction SilentlyContinue
        Get-Content -LiteralPath $err -Tail 8 -ErrorAction SilentlyContinue
        $live=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join [Environment]::NewLine
        if ($live -match 'failed to process shader|failed to resolve shader|error validating shader|ShaderValidationError|panicked at|Caught rendering error|OutOfMemory RenderError|Failed to send screenshot') {
            $process.Kill();$process.WaitForExit();throw 'Native renderer reported an error'
        }
        if ($process.WorkingSet64 -gt 6GB) {
            $process.Kill();$process.WaitForExit();throw 'Hosted software rendering exceeded the 6 GiB QA memory ceiling'
        }
        if ([DateTime]::UtcNow -gt $deadline) { $process.Kill();$process.WaitForExit();throw 'Hosted Windows native integration exceeded 15 minutes' }
    }
    if ($process.ExitCode -ne 0) { throw "Native Windows integration failed with exit code $($process.ExitCode)" }
    $logs=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join "`n"
    if ($logs -notmatch 'GRAPHICS:.*Microsoft Basic Render Driver.*Dx12') { throw 'Expected hosted software Direct3D 12 adapter was not observed' }
    if ($logs -match 'failed to process shader|failed to resolve shader|error validating shader|ShaderValidationError|panicked at|Caught rendering error|OutOfMemory RenderError|Failed to send screenshot') { throw 'Native Windows shader/runtime validation failed' }
    Add-Type -AssemblyName System.Drawing
    $imageAudit = @()
    foreach ($name in @('smoke','balanced','rebased','high','home','alpine','coast','map')) {
        $path="worldcraft-$name.png"
        if (-not (Test-Path -LiteralPath $path)) {throw "Missing Windows native capture $name"}
        $image=[System.Drawing.Bitmap]::FromFile((Join-Path $PWD $path))
        try {
            if ($image.Width -ne 128 -or $image.Height -ne 96) { throw "Unexpected Windows capture dimensions: $name" }
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
    if ($env:GH_TOKEN) {
        python tools/ci/publish_windows_audit.py
        if ($LASTEXITCODE -ne 0) { throw 'Windows render evidence could not be recorded' }
    }
    Write-Output 'Native Windows integration passed at a 128x96 software QA viewport. This is not a gaming performance test.'
} finally {
    # Keep complete logs and the memory history, including on a failed run. The
    # console tail alone concealed the earliest allocator failures previously.
    Copy-Item -LiteralPath $out -Destination 'native-windows.log' -ErrorAction SilentlyContinue
    Copy-Item -LiteralPath $err -Destination 'native-windows-errors.log' -ErrorAction SilentlyContinue
    @{scope='Hosted software DX12 integration; not gaming GPU profiling';peak_resident_bytes=$peakResidentBytes;samples=$memorySamples} |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath 'windows-render-resource-audit.json' -Encoding utf8
    Get-Content -LiteralPath $out -Tail 100 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 100 -ErrorAction SilentlyContinue
}
