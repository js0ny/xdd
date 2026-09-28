param(
  [string]$ExePath,
  [switch]$Global
)

$ErrorActionPreference = "Stop"
$scheme = "xdd"
if ($Global) {
  $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Global registration requires an elevated PowerShell session"
  }
}
$base = if ($Global) { "HKLM:\Software\Classes\$scheme" } else { "HKCU:\Software\Classes\$scheme" }
if (-not $ExePath) {
  $command = Get-Command xdd.exe -CommandType Application -ErrorAction Stop
  $ExePath = $command.Path
}

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
  throw "xdd executable not found: $ExePath"
}
$exe = (Resolve-Path -LiteralPath $ExePath -ErrorAction Stop).ProviderPath

New-Item -Path "$base\shell\open\command" -Force | Out-Null
New-ItemProperty `
  -Path $base `
  -Name "URL Protocol" `
  -Value "" `
  -PropertyType String `
  -Force | Out-Null

Set-ItemProperty `
  -Path $base `
  -Name "(default)" `
  -Value "URL:$scheme Protocol"

Set-ItemProperty `
  -Path "$base\shell\open\command" `
  -Name "(default)" `
  -Value "`"$exe`" open `"%1`""
