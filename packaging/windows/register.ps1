param([string]$ExePath)

$scheme = "xdd"
if (-not $ExePath) {
  $command = Get-Command xdd.exe -CommandType Application -ErrorAction Stop
  $ExePath = $command.Path
}

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
  throw "xdd executable not found: $ExePath"
}
$exe = (Resolve-Path -LiteralPath $ExePath -ErrorAction Stop).ProviderPath

New-Item -Path "HKCU:\Software\Classes\$scheme\shell\open\command" -Force | Out-Null
New-ItemProperty `
  -Path "HKCU:\Software\Classes\$scheme" `
  -Name "URL Protocol" `
  -Value "" `
  -PropertyType String `
  -Force | Out-Null

Set-ItemProperty `
  -Path "HKCU:\Software\Classes\$scheme" `
  -Name "(default)" `
  -Value "URL:$scheme Protocol"

Set-ItemProperty `
  -Path "HKCU:\Software\Classes\$scheme\shell\open\command" `
  -Name "(default)" `
  -Value "`"$exe`" open `"%1`""
