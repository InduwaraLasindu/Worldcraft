$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Join-Path $env:RUNNER_TEMP 'worldcraft-portable-visual'))
$permitted=[IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\')+'\'
if (-not $taskRoot.StartsWith($permitted,[StringComparison]::OrdinalIgnoreCase)) { throw 'Hosted extraction escaped temporary workspace' }
New-Item -ItemType Directory -Force -Path $taskRoot | Out-Null
gh release download $env:WORLDCRAFT_VISUAL_TAG --repo $env:GITHUB_REPOSITORY --pattern Worldcraft-Windows-x64-development.zip --dir $taskRoot
if ($LASTEXITCODE -ne 0) {throw 'Verified candidate download failed'}
$archive=Join-Path $taskRoot 'Worldcraft-Windows-x64-development.zip'
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $env:WORLDCRAFT_VISUAL_ZIP_SHA) {throw 'Candidate archive checksum mismatch'}
Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $taskRoot 'unpacked')
$binary=(Get-ChildItem -LiteralPath (Join-Path $taskRoot 'unpacked') -Filter Worldcraft.exe -File -Recurse | Select-Object -First 1).FullName
if (-not $binary) {throw 'Candidate executable is missing'}
$projectRoot=$PWD
$out=Join-Path $env:RUNNER_TEMP 'worldcraft-portable-visual-output.log'
$err=Join-Path $env:RUNNER_TEMP 'worldcraft-portable-visual-errors.log'
Remove-Item Env:WORLDCRAFT_ORT_DYLIB -ErrorAction SilentlyContinue
$env:WGPU_BACKEND='dx12'
$env:WGPU_DX12_COMPILER='dxc'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
Set-Location -LiteralPath (Split-Path -Parent $binary)
$env:WORLDCRAFT_VISUAL_CAPTURE=Join-Path $PWD 'worldcraft-learned.png'
$process=Start-Process -FilePath $binary -ArgumentList '--learned-smoke-test' -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
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
            found=SetWindowPos(window,IntPtr.Zero,0,0,320+(outer.Right-outer.Left)-(client.Right-client.Left),240+(outer.Bottom-outer.Top)-(client.Bottom-client.Top),0x0016);
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
Write-Output 'Hosted software viewport resized to 320x240. All scene and state assertions remain enabled.'


$deadline=[DateTime]::UtcNow.AddMinutes(10)
$passed=$false
try {
    while (-not $process.WaitForExit(15000)) {
        Get-Content -LiteralPath $out -Tail 8 -ErrorAction SilentlyContinue
        Get-Content -LiteralPath $err -Tail 8 -ErrorAction SilentlyContinue
        & python (Join-Path $projectRoot 'tools/ci/publish_portable_visual.py') running
        $live=((Get-Content -LiteralPath $out -Raw -ErrorAction SilentlyContinue),(Get-Content -LiteralPath $err -Raw -ErrorAction SilentlyContinue)) -join [Environment]::NewLine
        if ($live -match 'error validating shader|ShaderValidationError|panicked at|Caught rendering error|OutOfMemory RenderError|Failed to send screenshot' -or [DateTime]::UtcNow -gt $deadline) {
            $process.Kill();$process.WaitForExit();throw 'Packaged native rendering failed or exceeded its bounded timeout'
        }
    }
    if ($process.ExitCode -ne 0) {throw 'Packaged renderer returned an error'}
    $logs=((Get-Content -LiteralPath $out -Raw),(Get-Content -LiteralPath $err -Raw)) -join [Environment]::NewLine
    if ($logs -notmatch 'LEARNED AUDIT: native learned relief' -or $logs -match 'panicked at|Caught rendering error|OutOfMemory RenderError|Failed to send screenshot') {throw 'Packaged learned-world audit did not pass'}
    Add-Type -AssemblyName System.Drawing
    $picture=[System.Drawing.Bitmap]::FromFile($env:WORLDCRAFT_VISUAL_CAPTURE)
    try {
        if ($picture.Width -ne 320 -or $picture.Height -ne 240) {throw 'Unexpected portable capture size'}
        $colours=[System.Collections.Generic.HashSet[int]]::new()
        $values=[System.Collections.Generic.List[double]]::new()
        for ($y=85;$y -lt 200;$y+=3) {
            for ($x=15;$x -lt 305;$x+=3) {
                $pixel=$picture.GetPixel($x,$y)
                [void]$colours.Add((($pixel.R -shr 3)*1024)+(($pixel.G -shr 3)*32)+($pixel.B -shr 3))
                $values.Add(($pixel.R+$pixel.G+$pixel.B)/3.0)
            }
        }
        $stats=$values | Measure-Object -Minimum -Maximum
        if ($colours.Count -lt 20 -or ($stats.Maximum-$stats.Minimum) -lt 15) {throw 'Central scene crop is unexpectedly uniform'}
        @{width=320;height=240;central_colours=$colours.Count;central_brightness_range=($stats.Maximum-$stats.Minimum)} |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $env:RUNNER_TEMP 'worldcraft-portable-visual-pixels.json') -Encoding utf8
    } finally {$picture.Dispose()}
    $passed=$true
} finally {
    Get-Content -LiteralPath $out -Tail 100 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 100 -ErrorAction SilentlyContinue
    if ($passed) { & python (Join-Path $projectRoot 'tools/ci/publish_portable_visual.py') passed_image_checks }
    else { & python (Join-Path $projectRoot 'tools/ci/publish_portable_visual.py') failed }
    Set-Location -LiteralPath $projectRoot
}
if (-not $passed -or $LASTEXITCODE -ne 0) {throw 'Portable visual verification failed'}
