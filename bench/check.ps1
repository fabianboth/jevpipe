#Requires -Version 7.3
[CmdletBinding()]
param(
    [switch]$Fix,
    [string]$Filter,
    [ValidateSet('format', 'lint', 'types', 'test')]
    [string[]]$Stage = @('format', 'lint', 'types', 'test')
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
    if ($Fix) { uv run @locked ruff format } else { uv run @locked ruff format --check }
}

Invoke-Stage 'lint' {
    if ($Fix) { uv run @locked ruff check --fix } else { uv run @locked ruff check }
}

Invoke-Stage 'types' {
    uv run @locked pyright
}

Invoke-Stage 'test' {
    if ($Filter) { uv run @locked pytest -k $Filter } else { uv run @locked pytest }
}

Write-Host 'All checks passed' -ForegroundColor Green
