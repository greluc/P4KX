<#
.SYNOPSIS
Installs an MSI per user, upgrades it with a newer one and uninstalls it, failing on any deviation.

.PARAMETER Msi
The MSI of the Release.

.PARAMETER NextMsi
The same installer built with a higher version.

.PARAMETER NextVersion
The version of NextMsi.

.PARAMETER UpgradeCode
The UpgradeCode both installers share.
#>
param(
    [Parameter(Mandatory)] [string] $Msi,
    [Parameter(Mandatory)] [string] $NextMsi,
    [Parameter(Mandatory)] [string] $NextVersion,
    [Parameter(Mandatory)] [string] $UpgradeCode
)

$ErrorActionPreference = 'Stop'
$folder = Join-Path $env:LOCALAPPDATA 'Programs\P4K-Extractor'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\P4K-Extractor.lnk'
$installer = New-Object -ComObject WindowsInstaller.Installer

function Invoke-Msiexec([string[]] $Arguments, [string] $Log) {
    $process = Start-Process msiexec.exe -ArgumentList ($Arguments + @('/qn', '/l*v', $Log)) -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Get-Content $Log -Tail 60
        throw "msiexec $($Arguments -join ' ') failed with exit code $($process.ExitCode)"
    }
}

function Get-Installed {
    @($installer.RelatedProducts("{$UpgradeCode}"))
}

Invoke-Msiexec @('/i', $Msi) 'install.log'
foreach ($path in @((Join-Path $folder 'p4kx-gui.exe'), (Join-Path $folder 'p4kx.exe'), $shortcut)) {
    if (-not (Test-Path $path)) { throw "missing after install: $path" }
}
if ((Get-Installed).Count -ne 1) { throw 'the install is not registered once' }

Invoke-Msiexec @('/i', $NextMsi) 'upgrade.log'
$installed = Get-Installed
if ($installed.Count -ne 1) { throw "the upgrade left $($installed.Count) installations" }
$version = $installer.ProductInfo($installed[0], 'VersionString')
if ($version -ne $NextVersion) { throw "the upgrade installed $version instead of $NextVersion" }

Invoke-Msiexec @('/x', $installed[0]) 'uninstall.log'
if ((Get-Installed).Count -ne 0) { throw 'still installed after uninstall' }
foreach ($path in @($folder, $shortcut)) {
    if (Test-Path $path) { throw "left behind after uninstall: $path" }
}
'install, upgrade and uninstall work'
