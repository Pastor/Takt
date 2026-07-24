<#
.SYNOPSIS
  install-rustrover-plugin.ps1 - build the Takt IntelliJ Platform plugin and
  install/update it into detected JetBrains RustRover installations (Windows).

.DESCRIPTION
  Steps:
    1. Builds the plugin in extensions\intellij-takt (its own gradlew.bat -> buildPlugin),
       producing build\distributions\intellij-takt-<version>.zip.
    2. Locates RustRover plugin directories in the standard JetBrains location on
       Windows (%APPDATA%\JetBrains\RustRover*; including Toolbox installs - the
       config layout there is standard).
    3. Installs the plugin, or updates it if a previous version exists (removes the
       old plugin folder and unpacks the fresh one).

  After installation restart RustRover (plugins are picked up on startup). If the
  IDE is running, changes apply after a restart.

.PARAMETER SkipBuild
  Do not rebuild - use the ready zip from build\distributions.

.EXAMPLE
  extensions\install-rustrover-plugin.ps1              # build and install/update

.EXAMPLE
  extensions\install-rustrover-plugin.ps1 -SkipBuild   # no rebuild (use existing zip)
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Script root = extensions\; the plugin project sits next to it.
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PluginDir = Join-Path $ScriptDir 'intellij-takt'
$GradlewBat = Join-Path $PluginDir 'gradlew.bat'

if (-not (Test-Path -LiteralPath $PluginDir -PathType Container)) {
    Write-Error "Plugin project not found: $PluginDir"; exit 1
}
if (-not (Test-Path -LiteralPath $GradlewBat -PathType Leaf)) {
    Write-Error "Not found: $GradlewBat"; exit 1
}

# --- 1. Build --------------------------------------------------------------
if (-not $SkipBuild) {
    Write-Host '==> Building the plugin (buildPlugin)...'
    & $GradlewBat "-p" $PluginDir '--console=plain' 'buildPlugin'
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Plugin build failed (exit code $LASTEXITCODE)."; exit 1
    }
} else {
    Write-Host '==> Build skipped (-SkipBuild).'
}

# --- 2. Find the built zip -------------------------------------------------
$DistDir = Join-Path $PluginDir 'build\distributions'
$Version = $null
$propsPath = Join-Path $PluginDir 'gradle.properties'
$verLine = Select-String -LiteralPath $propsPath -Pattern '^pluginVersion\s*=\s*(.+?)\s*$' |
    Select-Object -First 1
if ($verLine) { $Version = $verLine.Matches[0].Groups[1].Value }

$Zip = $null
if ($Version) {
    $candidate = Join-Path $DistDir "intellij-takt-$Version.zip"
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { $Zip = $candidate }
}
if (-not $Zip) {
    # Fallback: newest zip in distributions.
    $Zip = Get-ChildItem -LiteralPath $DistDir -Filter '*.zip' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Zip -or -not (Test-Path -LiteralPath $Zip -PathType Leaf)) {
    Write-Error "No built plugin found in $DistDir (run without -SkipBuild)."; exit 1
}

# Plugin folder name inside the zip (top path component of the first archive entry).
Add-Type -AssemblyName System.IO.Compression.FileSystem
$PluginName = $null
$archive = [System.IO.Compression.ZipFile]::OpenRead($Zip)
try {
    $firstEntry = $archive.Entries | Select-Object -First 1
    if ($firstEntry) { $PluginName = ($firstEntry.FullName -split '[\\/]')[0] }
} finally {
    $archive.Dispose()
}
if (-not $PluginName) { Write-Error "Could not determine plugin name from $Zip"; exit 1 }
Write-Host "==> Plugin: $PluginName (from $(Split-Path -Leaf $Zip))"

# --- 3. RustRover plugin directories ---------------------------------------
# On Windows the JetBrains config is %APPDATA%\JetBrains\RustRover<ver>, plugins
# live in the plugins subfolder. Iterate over every detected version.
$JbBase = Join-Path $env:APPDATA 'JetBrains'
if (-not (Test-Path -LiteralPath $JbBase -PathType Container)) {
    Write-Error "JetBrains directory not found: $JbBase"; exit 1
}

# Running-IDE hint - a rustrover* process.
$Running = @(Get-Process -ErrorAction SilentlyContinue |
    Where-Object { $_.ProcessName -like 'rustrover*' }).Count -gt 0

$Installed = 0
$RustRovers = Get-ChildItem -LiteralPath $JbBase -Directory -Filter 'RustRover*' -ErrorAction SilentlyContinue
foreach ($RR in $RustRovers) {
    $PluginsDir = Join-Path $RR.FullName 'plugins'
    New-Item -ItemType Directory -Force -Path $PluginsDir | Out-Null

    $Target = Join-Path $PluginsDir $PluginName
    $Action = if (Test-Path -LiteralPath $Target) { 'updated' } else { 'installed' }
    if (Test-Path -LiteralPath $Target) { Remove-Item -LiteralPath $Target -Recurse -Force }

    Expand-Archive -LiteralPath $Zip -DestinationPath $PluginsDir -Force

    Write-Host "==> [$($RR.Name)] plugin $Action -> $Target"
    $Installed++
}

if ($Installed -eq 0) {
    Write-Error @"
RustRover not found in $JbBase (no RustRover* directories).
Install RustRover and launch it at least once, then retry.
"@
    exit 1
}

Write-Host "==> Done: installations updated - $Installed."
if ($Running) {
    Write-Host '!!  RustRover seems to be running - restart the IDE to apply the plugin.'
} else {
    Write-Host '    Start/restart RustRover to load the plugin.'
}
