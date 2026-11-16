$ErrorActionPreference='Stop'
foreach ($scriptFile in (Get-ChildItem -LiteralPath tools -Filter '*.ps1' -File)) {
    $parseTokens=$null
    $parseErrors=$null
    $null=[System.Management.Automation.Language.Parser]::ParseFile($scriptFile.FullName,[ref]$parseTokens,[ref]$parseErrors)
    if ($parseErrors.Count -gt 0) { $parseErrors | Format-List;throw "PowerShell syntax errors in $($scriptFile.Name)" }
    Write-Output "Parsed $($scriptFile.Name)"
}
$inno=Get-Command ISCC.exe -ErrorAction SilentlyContinue
$path=if($inno){$inno.Source}else{Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'}
Write-Output "Hosted installer compiler present: $(Test-Path -LiteralPath $path)"
