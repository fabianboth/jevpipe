#Requires -Version 7.3
[CmdletBinding()]
param(
    [switch]$Fix,
    [string]$Filter,
    [ValidateSet('format', 'lint', 'test', 'deny')]
    [string[]]$Stage = @('format', 'lint', 'test', 'deny')
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
Set-Location $PSScriptRoot

$locked = @(if (-not $Fix) { '--locked' })

function Invoke-Stage([string]$Name, [scriptblock]$Command) {
    if ($Name -notin $Stage) { return }
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
}

Invoke-Stage 'format' {
    if ($Fix) { cargo fmt --all } else { cargo fmt --all --check }
}

Invoke-Stage 'lint' {
    cargo clippy --all-targets @locked -- -D warnings
}

Invoke-Stage 'test' {
    if ($Filter) { cargo test @locked -- $Filter } else { cargo test @locked }
}

Invoke-Stage 'deny' {
    cargo deny @locked check
}

Write-Host 'All checks passed' -ForegroundColor Green
