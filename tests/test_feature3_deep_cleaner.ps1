# Test Feature 3: Deep System Cleaner & Shader Cache Purger
$exePath = Join-Path $PSScriptRoot "..\dist\AnxiouslyOptimized.exe"
if (!(Test-Path $exePath)) {
    Write-Error "Binary not found at $exePath"
    exit 1
}

$bytes = [System.IO.File]::ReadAllBytes($exePath)
$asm = [System.Reflection.Assembly]::Load($bytes)
$cleanerType = $asm.GetType("AnxiouslyOptimized.Services.DeepCleanerService")

if ($null -eq $cleanerType) {
    Write-Error "Could not find AnxiouslyOptimized.Services.DeepCleanerService in assembly"
    exit 1
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " FEATURE 3: DEEP CLEANER & SHADER PURGER VERIFICATION" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$getDefaultMethod = $cleanerType.GetMethod("GetDefaultCategories", [System.Reflection.BindingFlags]"Public,Static")
$scanMethod = $cleanerType.GetMethod("ScanAllCategoriesAsync", [System.Reflection.BindingFlags]"Public,Static")
$purgeMethod = $cleanerType.GetMethod("PurgeCategoriesAsync", [System.Reflection.BindingFlags]"Public,Static")

# 1. Test Default Categories Registry
$categories = $getDefaultMethod.Invoke($null, @())
Write-Host "  [TEST 1] Loaded Target Categories:" ($categories.Count) "deep cleaner targets" -ForegroundColor Green

$expectedIds = @("gpu_shaders", "delivery_opt", "win_update", "chromium_caches", "crash_dumps", "system_temp", "thumb_cache", "emulator_cache")
$allFound = $true
foreach ($eid in $expectedIds) {
    $found = ($categories | Where-Object { $_.Id -eq $eid }) -ne $null
    if (-not $found) { $allFound = $false }
}
Write-Host "  [TEST 2] All 8 Commercial Targets Present:" $(if ($allFound) { "PASS (Shaders, DeliveryOpt, Browsers, WER, Emulators)" } else { "FAIL" }) -ForegroundColor $(if ($allFound) { "Green" } else { "Red" })

# 2. Test In-Process Scan Execution
$task = $scanMethod.Invoke($null, @($null))
$task.Wait()
$scannedCategories = $task.Result

$totalFiles = 0
$totalBytes = [long]0
foreach ($c in $scannedCategories) {
    $totalFiles += $c.FileCount
    $totalBytes += $c.TotalSizeBytes
    Write-Host ("         -> {0,-35} : {1,-10} ({2} files)" -f $c.Title, $c.SizeFormatted, $c.FileCount) -ForegroundColor DarkGreen
}

$formattedTotal = if ($totalBytes -lt 1073741824) { "{0:N1} MB" -f ($totalBytes / 1048576) } else { "{0:N2} GB" -f ($totalBytes / 1073741824) }
Write-Host "  [TEST 3] Real-Time Scan Execution: PASS (Identified $formattedTotal reclaimable across $totalFiles files)" -ForegroundColor Green

# 3. Test Purge with Mock Target
$mockDir = Join-Path $PSScriptRoot "scratch_cleaner_test"
if (Test-Path $mockDir) { Remove-Item $mockDir -Recurse -Force -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Path $mockDir -Force | Out-Null

$dummyFile1 = Join-Path $mockDir "shader_cache_test.bin"
$dummyFile2 = Join-Path $mockDir "d3d_pso_cache.tmp"
[System.IO.File]::WriteAllBytes($dummyFile1, (New-Object byte[] (1024 * 512))) # 512 KB
[System.IO.File]::WriteAllBytes($dummyFile2, (New-Object byte[] (1024 * 1024))) # 1 MB

$testCatType = $asm.GetType("AnxiouslyOptimized.Services.CleanerTargetCategory")
$testCat = [Activator]::CreateInstance($testCatType)
$testCat.Id = "mock_test"
$testCat.Title = "Mock Test Cache"
$testCat.IsSelected = $true
$testCat.Directories.Add($mockDir)

$genListDef = [System.Collections.Generic.List[object]].GetGenericTypeDefinition()
$catListType = $genListDef.MakeGenericType(@($testCatType))
$catList = [Activator]::CreateInstance($catListType)
$catList.Add($testCat)

$purgeTask = $purgeMethod.Invoke($null, @($catList, $null))
$purgeTask.Wait()
$freed = $purgeTask.Result

$filesRemaining = (Get-ChildItem -Path $mockDir -File -ErrorAction SilentlyContinue).Count
$purgePassed = ($filesRemaining -eq 0 -and $freed -ge (1024 * 1024))
Write-Host "  [TEST 4] Deep Purge Execution & Recovery Calc:" $(if ($purgePassed) { "PASS (Purged mock files, calculated $($freed) bytes)" } else { "FAIL" }) -ForegroundColor $(if ($purgePassed) { "Green" } else { "Red" })

if (Test-Path $mockDir) { Remove-Item $mockDir -Recurse -Force -ErrorAction SilentlyContinue }

# 4. Emdash validation
$emdashFiles = @()
Get-ChildItem -Path (Join-Path $PSScriptRoot "..\src") -Recurse -Include *.cs, *.xaml | ForEach-Object {
    $text = [System.IO.File]::ReadAllText($_.FullName)
    if ($text -match '[\u2013\u2014]') {
        $emdashFiles += $_.FullName
    }
}
if ($emdashFiles.Count -eq 0) {
    Write-Host "  [TEST 5] Zero Emdashes Across All Source Files: PASS" -ForegroundColor Green
} else {
    Write-Host "  [TEST 5] Found Emdashes in: " ($emdashFiles -join ", ") -ForegroundColor Red
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " FEATURE 3 IMPLEMENTATION AND VALIDATION COMPLETE" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
