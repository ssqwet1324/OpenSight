#Requires -RunAsAdministrator
param([Parameter(Mandatory = $true)][string]$LocalAddress)

$ErrorActionPreference = 'Stop'
$opensightExecutable = Join-Path (Split-Path -Parent $PSScriptRoot) '.local\opensight.exe'
if (-not (Test-Path -LiteralPath $opensightExecutable)) {
    throw 'Build lensService/.local/opensight.exe first.'
}

$opensightRuleName = 'OpenSight-Go-LAN-8080'
if (-not (Get-NetFirewallRule -Name $opensightRuleName -ErrorAction SilentlyContinue)) {
    New-NetFirewallRule -Name $opensightRuleName `
        -DisplayName 'OpenSight Go backend (LAN TCP 8080)' `
        -Direction Inbound -Action Allow -Protocol TCP -LocalPort 8080 `
        -LocalAddress $LocalAddress -RemoteAddress LocalSubnet -Profile Any `
        -Program $opensightExecutable -EdgeTraversalPolicy Block | Out-Null
}
Get-NetFirewallRule -Name $opensightRuleName | Select-Object Name, Enabled, Action
