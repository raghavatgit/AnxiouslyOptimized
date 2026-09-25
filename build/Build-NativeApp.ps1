# ==============================================================================
#  ANXIOUSLYOPTIMIZED - NATIVE C# WPF STANDALONE BUILD SCRIPT
#  Compiles native zero-dependency PE binary targeting .NET Framework 4.8
# ==============================================================================

$baseDir = Split-Path -Parent $PSScriptRoot
if (!$baseDir) { $baseDir = (Get-Location).Path }
Set-Location $baseDir

$distDir = Join-Path $baseDir "dist"
if (!(Test-Path $distDir)) {
    New-Item -Path $distDir -ItemType Directory -Force | Out-Null
}

# Terminate any running instance of the application to prevent file locking
Get-Process -Name "AnxiouslyOptimized" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

$distExe = Join-Path $distDir "AnxiouslyOptimized.exe"
if (Test-Path $distExe) {
    try {
        Remove-Item $distExe -Force -ErrorAction Stop
    } catch {
        # If locked by an elevated process, rename it so MSBuild can output fresh binary
        $tmpName = "AnxiouslyOptimized.old." + [Guid]::NewGuid().ToString("N").Substring(0,8) + ".exe"
        Rename-Item -Path $distExe -NewName $tmpName -Force -ErrorAction SilentlyContinue
    }
}

$projectPath = Join-Path $baseDir "src\AnxiouslyOptimized.csproj"
$msbuildPath = "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\MSBuild.exe"

if (!(Test-Path $msbuildPath)) {
    Write-Error "MSBuild.exe was not found at $msbuildPath"
    exit 1
}

Write-Host @"
 __________________________________________________________________
|                                                                  |
|   ANXIOUSLYOPTIMIZED - NATIVE C# WPF COMPILATION PIPELINE        |
|   Zero-Dependency, Direct3D-Accelerated Commercial Executable    |
|__________________________________________________________________|
"@ -ForegroundColor Cyan

Write-Host "`n[1/2] Invoking MSBuild Engine..." -ForegroundColor Yellow
$sw = [System.Diagnostics.Stopwatch]::StartNew()

$msbuildArgs = @(
    "`"$projectPath`"",
    "/p:Configuration=Release",
    "/p:Platform=x64",
    "/v:minimal",
    "/nologo"
)

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $msbuildPath
$psi.Arguments = [string]::Join(" ", $msbuildArgs)
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true

$proc = [System.Diagnostics.Process]::Start($psi)
$stdout = $proc.StandardOutput.ReadToEnd()
$stderr = $proc.StandardError.ReadToEnd()
$proc.WaitForExit()

$sw.Stop()

Write-Host $stdout

if ($proc.ExitCode -ne 0) {
    Write-Host $stderr -ForegroundColor Red
    Write-Error "Compilation failed with exit code $($proc.ExitCode)"
    exit $proc.ExitCode
}

$outExe = Join-Path $distDir "AnxiouslyOptimized.exe"
if (Test-Path $outExe) {
    $info = Get-Item $outExe
    $sizeMb = [math]::Round($info.Length / (1024 * 1024), 2)
    Write-Host "`n[2/2] Verification:" -ForegroundColor Green
    Write-Host "  Executable : $($info.FullName)" -ForegroundColor White
    Write-Host "  Size       : $sizeMb MB ($($info.Length) bytes)" -ForegroundColor White
    Write-Host "  Build Time : $($sw.ElapsedMilliseconds) ms" -ForegroundColor White
    Write-Host "`n-------------------------------------------------------------------" -ForegroundColor Cyan
    Write-Host "  BUILD SUCCESS - NATIVE STANDALONE READY IN DIST/" -ForegroundColor Green
    Write-Host "-------------------------------------------------------------------`n" -ForegroundColor Cyan
} else {
    Write-Error "Expected binary $outExe was not found after build."
    exit 1
}
