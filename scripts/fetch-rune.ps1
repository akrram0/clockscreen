# Rune builds against crates patched inside its own repo, so a checkout of the pinned rev has to
# sit in .rune/ before cargo runs (see [patch.crates-io] in Cargo.toml).
$ErrorActionPreference = 'Stop'
$rev = '53baebed87f3a64d081c3cebedb0482aed191558'
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root '.rune'

if (Test-Path (Join-Path $target 'vendor\wgpu-hal\Cargo.toml')) {
    Write-Host "rune checkout already present at $target"
    exit 0
}

New-Item -ItemType Directory -Force -Path $target | Out-Null
git -C $target init -q
git -C $target remote add origin https://github.com/anomalyco/rune 2>$null
git -C $target fetch --depth 1 origin $rev
git -C $target checkout -q FETCH_HEAD
Write-Host "rune $rev checked out at $target"
