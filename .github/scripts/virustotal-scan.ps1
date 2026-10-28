<#
.SYNOPSIS
Uploads release assets to VirusTotal and writes a Markdown "Security check" section.

.DESCRIPTION
Hashes every file given with -Path (SHA-256), looks the hash up on VirusTotal, uploads the file
when VirusTotal does not know it yet, waits for the analysis and writes one table row per file
(detections and a report link) to -NotePath. Works on Windows and Linux with PowerShell 7.

The scan never blocks a release. A missing API key, a rejected upload or a timeout is
reported as a warning and as a sentence in the note; the script then exits with 0.

.PARAMETER Path
The files to scan.

.PARAMETER NotePath
Where the Markdown section is written. It is always written, also when the scan did not run.

.PARAMETER ApiKey
The VirusTotal API key. Defaults to the environment variable VIRUSTOTAL_API_KEY.

.PARAMETER TimeoutMinutes
How long to wait for each analysis before the report is linked as pending.

.PARAMETER PollSeconds
The pause between two polls of an analysis.

.EXAMPLE
./virustotal-scan.ps1 -Path P4KX-1.0.0-x86_64.msi, P4KX-1.0.0-x86_64-portable.zip -NotePath virustotal.md
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string[]]$Path,
    [Parameter(Mandatory)][string]$NotePath,
    [string]$ApiKey = $env:VIRUSTOTAL_API_KEY,
    [int]$TimeoutMinutes = 10,
    [int]$PollSeconds = 30
)

$ErrorActionPreference = 'Stop'

$api = 'https://www.virustotal.com/api/v3'
$directUploadLimit = 32MB
$uploadTimeoutSeconds = 300
$maxReasonLength = 120
$heading = '## Security check'

function Write-Note([string]$Body) {
    Set-Content -LiteralPath $NotePath -Value $Body -Encoding utf8NoBOM
}

# Keeps a message safe for public release notes: plain characters only, no key, capped length.
function Get-SafeText([string]$Text) {
    if ($ApiKey) { $Text = $Text.Replace($ApiKey, '') }
    $plain = ($Text -replace '[^A-Za-z0-9 .,:;()/_-]', ' ' -replace '\s+', ' ').Trim()
    if ($plain.Length -gt $maxReasonLength) { $plain = $plain.Substring(0, $maxReasonLength) + '...' }
    $plain
}

function Write-Unavailable([string]$Reason) {
    $safe = Get-SafeText $Reason
    Write-Host "::warning title=VirusTotal::$safe - the release notes say the scan did not run."
    Write-Note "$heading`n`nThe VirusTotal scan did not run for this release ($safe)."
    exit 0
}

trap { Write-Unavailable "unexpected error: $($_.Exception.Message)" }

function Invoke-Vt {
    param([string]$Uri, [int]$TimeoutSec = 120)
    Invoke-RestMethod -Uri $Uri -Method Get -TimeoutSec $TimeoutSec -Headers @{
        'x-apikey' = $ApiKey
        'accept'   = 'application/json'
    }
}

function Get-HttpStatus($ErrorRecord) {
    if ($ErrorRecord.Exception.Response) { return [int]$ErrorRecord.Exception.Response.StatusCode }
    return 0
}

function Send-MultipartFile([string]$Uri, [System.IO.FileInfo]$File) {
    $keyConfig = 'header = "x-apikey: {0}"' -f $ApiKey
    $lines = $keyConfig | & curl --config - `
        --silent --show-error --fail-with-body `
        --max-time $uploadTimeoutSeconds `
        --header 'accept: application/json' `
        --form "file=@$($File.FullName)" `
        --write-out '\n%{http_code}' `
        --url $Uri
    $exit = $LASTEXITCODE
    $all = @($lines)
    $status = 0
    if ($all.Count) { [int]::TryParse(("$($all[-1])").Trim(), [ref]$status) | Out-Null }
    $body = if ($all.Count -gt 1) { ($all[0..($all.Count - 2)]) -join "`n" } else { '' }
    [pscustomobject]@{
        Ok       = ($exit -eq 0 -and $status -ge 200 -and $status -lt 300)
        Status   = $status
        Body     = $body
        CurlExit = $exit
    }
}

function Get-FileStats([System.IO.FileInfo]$File, [string]$Sha256) {
    $report = $null
    try { $report = Invoke-Vt "$api/files/$Sha256" }
    catch { if ((Get-HttpStatus $_) -ne 404) { throw } }
    if ($report) { return $report.data.attributes.last_analysis_stats }

    $uploadUrl = if ($File.Length -le $directUploadLimit) { "$api/files" } else { (Invoke-Vt "$api/files/upload_url").data }
    Write-Host "Uploading $($File.Name)..."
    $response = Send-MultipartFile $uploadUrl $File
    if (-not $response.Ok) {
        $why = if ($response.Status) { "HTTP $($response.Status)" } else { "curl exit $($response.CurlExit)" }
        throw "upload rejected ($why)"
    }
    $analysisId = ($response.Body | ConvertFrom-Json).data.id

    $deadline = (Get-Date).AddMinutes($TimeoutMinutes)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds $PollSeconds
        try { $analysis = Invoke-Vt "$api/analyses/$analysisId" }
        catch {
            Write-Host "  poll failed ($($_.Exception.Message)) - retrying."
            continue
        }
        Write-Host "  $($File.Name): $($analysis.data.attributes.status)"
        if ($analysis.data.attributes.status -eq 'completed') { return $analysis.data.attributes.stats }
    }
    return $null
}

if (-not $ApiKey) { Write-Unavailable 'VIRUSTOTAL_API_KEY is not set' }

$rows = foreach ($item in $Path) {
    $file = Get-Item -LiteralPath $item -ErrorAction SilentlyContinue
    if (-not $file) { Write-Unavailable "no file at '$item'" }
    $sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $link = "[report](https://www.virustotal.com/gui/file/$sha256)"
    try { $stats = Get-FileStats $file $sha256 }
    catch {
        $status = Get-HttpStatus $_
        $hint = if ($status -eq 401) { 'the API key was rejected' }
                elseif ($status -eq 429) { 'the API quota is exhausted' }
                else { Get-SafeText $_.Exception.Message }
        Write-Host "::warning title=VirusTotal::$($file.Name): $hint"
        "| ``$($file.Name)`` | not scanned ($hint) | $link |"
        continue
    }
    $flagged = [int]$stats.malicious + [int]$stats.suspicious
    $engines = $flagged + [int]$stats.undetected + [int]$stats.harmless
    if (-not $stats -or $engines -eq 0) {
        Write-Host "::warning title=VirusTotal::$($file.Name): no analysis result yet."
        "| ``$($file.Name)`` | analysis pending | $link |"
        continue
    }
    Write-Host "$($file.Name): $flagged / $engines engines flagged the file."
    "| ``$($file.Name)`` | $flagged of $engines engines | $link |"
}

Write-Note @"
$heading

Every binary asset was uploaded to VirusTotal straight after the CI build.

| File | Detections | Report |
|:-----|:-----------|:-------|
$($rows -join "`n")

An unsigned installer or archive that is new on every release can draw individual false
positives; the report shows which engine says what.
"@
