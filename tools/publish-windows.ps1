$ErrorActionPreference='Stop'
$ready = $false
for ($attempt = 0; $attempt -lt 120; $attempt++) {
  $jobs = (gh api "repos/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID/jobs" | ConvertFrom-Json).jobs
  $linux = $jobs | Where-Object { $_.name -eq 'desktop (ubuntu-latest)' }
  if ($linux.status -eq 'completed') {
    $core = @($jobs | Where-Object { $_.name -like 'core (*)' })
    $ready = $linux.conclusion -eq 'success' -and $core.Count -eq 2 -and @($core | Where-Object { $_.conclusion -ne 'success' }).Count -eq 0
    break
  }
  Start-Sleep -Seconds 15
}
if (-not $ready) { Write-Output 'Packaging skipped: native Linux verification has not passed.'; exit 0 }
./tools/package-windows.ps1
if ($LASTEXITCODE -ne 0) { throw 'Packaging failed' }
$tag = "v$((cargo metadata --locked --format-version 1 | ConvertFrom-Json).packages | Where-Object name -eq 'worldcraft' | Select-Object -ExpandProperty version)-build.$env:GITHUB_RUN_NUMBER"
$source = git rev-parse HEAD
$installerAsset = (Get-ChildItem -LiteralPath dist -Filter 'Worldcraft-Setup-*.exe' -File | Select-Object -First 1).FullName
gh release create $tag --repo $env:GITHUB_REPOSITORY --draft --prerelease --target $source --title "Worldcraft development build $tag" --notes-file dist/Worldcraft-Windows-x64-development/BUILD-INFO.txt dist/Worldcraft-Windows-x64-development.zip $installerAsset dist/SHA256SUMS.txt
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare development build' }
