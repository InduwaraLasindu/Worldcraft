param([ValidateSet('release','ci')][string]$Profile='release', [switch]$Relief)
$ErrorActionPreference='Stop'
$env:WGPU_BACKEND='dx12'
$env:WGPU_DX12_COMPILER='dxc'
$env:WGPU_FORCE_FALLBACK_ADAPTER='1'
$route = if ($Relief) {'material-relief'} else {'render-settings'}
$flag = if ($Relief) {'--material-relief-test'} else {'--render-smoke-test'}
$marker = if ($Relief) {'RELIEF AUDIT:'} else {'RENDER AUDIT:'}
$pattern = if ($Relief) {'worldcraft-relief-*.png'} else {'worldcraft-render-*.png'}
$report = if ($Relief) {'relief-render-audit.json'} else {'renderer-settings-audit.json'}
$out=Join-Path $env:RUNNER_TEMP "worldcraft-$route.log"
$err=Join-Path $env:RUNNER_TEMP "worldcraft-$route-errors.log"
Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue | Remove-Item
Remove-Item -LiteralPath $report -ErrorAction SilentlyContinue
$process=Start-Process -FilePath (Join-Path $PWD "target/$Profile/worldcraft.exe") -ArgumentList $flag -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
# Bound fill cost on WARP and present the QA window without activating it.
# This changes only the hosted test process; normal application defaults stay intact.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class WorldcraftRenderSettingsQaWindow {
    public delegate bool Callback(IntPtr handle,IntPtr data);
    [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback,IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window,out uint process);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr window,StringBuilder text,int length);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window,out Rect rect);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window,out Rect rect);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window,IntPtr after,int x,int y,int width,int height,uint flags);
    public static bool Resize(uint process) {
        bool found=false;
        EnumWindows((window,data)=>{
            uint owner;GetWindowThreadProcessId(window,out owner);
            if(owner!=process)return true;
            var title=new StringBuilder(256);GetWindowText(window,title,256);
            if(!title.ToString().StartsWith("Worldcraft"))return true;
            Rect client,outer;GetClientRect(window,out client);GetWindowRect(window,out outer);
            found=SetWindowPos(window,IntPtr.Zero,0,0,400+(outer.Right-outer.Left)-(client.Right-client.Left),300+(outer.Bottom-outer.Top)-(client.Bottom-client.Top),0x0056);
            return false;
        },IntPtr.Zero);
        return found;
    }
}
'@
$resizeDeadline=[DateTime]::UtcNow.AddSeconds(20)
$resized=$false
while (-not $process.HasExited -and [DateTime]::UtcNow -lt $resizeDeadline) {
    if ([WorldcraftRenderSettingsQaWindow]::Resize([uint32]$process.Id)) {$resized=$true;break}
    Start-Sleep -Milliseconds 100
}
if (-not $resized) {if (-not $process.HasExited) {$process.Kill()};throw 'Hosted render-settings viewport was not found'}
Write-Output 'Hosted software QA viewport: 400x300, shown without activation. Shader and memory gates are enabled.'
$deadline=[DateTime]::UtcNow.AddMinutes(30)
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
    if ($logs -match $hardErrors -or $logs -notmatch $marker) {throw 'Render-settings shader/state gate failed'}
    if ($Relief) {python tools/ci/check_relief_images.py --rock-only} else {python tools/ci/check_render_images.py}
    if ($LASTEXITCODE -ne 0) {throw 'Render-settings pixel/ownership gate failed'}
} finally {
    Copy-Item -LiteralPath $out -Destination "native-$route.log" -ErrorAction SilentlyContinue
    Copy-Item -LiteralPath $err -Destination "native-$route-errors.log" -ErrorAction SilentlyContinue
    @{source=$env:GITHUB_SHA;peak_resident_bytes=$peak;scope='Hosted software DX12 at 400x300; unsupported vendor dispatches are skips'} |
        ConvertTo-Json | Set-Content -LiteralPath "windows-$route-resource-audit.json" -Encoding utf8
    Get-Content -LiteralPath $out -Tail 50 -ErrorAction SilentlyContinue
    Get-Content -LiteralPath $err -Tail 50 -ErrorAction SilentlyContinue
}
