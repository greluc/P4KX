<#
.SYNOPSIS
Builds and validates the P4K-Extractor MSI with WiX Toolset 7 from release binaries.

.PARAMETER Wix
The wix.exe of WiX Toolset 7.

.PARAMETER Version
The installer version, three numbers such as 0.1.0.

.PARAMETER Output
The MSI to write.

.PARAMETER BinaryFolder
The folder holding p4kx-gui.exe, p4kx.exe and p4kx-mcp.exe.
#>
param(
    [Parameter(Mandatory)] [string] $Wix,
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $Output,
    [string] $BinaryFolder = 'target\release'
)

$ErrorActionPreference = 'Stop'
$repository = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$source = Join-Path $repository 'crates\p4kx-gui\wix\main.wxs'

& $Wix build $source -acceptEula wix7 -arch x64 `
    -d "Version=$Version" `
    -d "BinaryFolder=$(Resolve-Path $BinaryFolder)" `
    -d "RepositoryFolder=$repository" `
    -o $Output
if ($LASTEXITCODE -ne 0) { throw "wix build failed with exit code $LASTEXITCODE" }

& $Wix msi validate -acceptEula wix7 $Output
if ($LASTEXITCODE -ne 0) { throw "wix msi validate failed with exit code $LASTEXITCODE" }
