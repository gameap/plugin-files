# Brings up the Windows leg: the panel and the node both live on the runner.
# Exports E2E_NODE_HOST, PANEL_TAG and GAMEAPCTL_TAG through $env:GITHUB_ENV.
#
# The daemon is installed by gameapctl for one concrete reason beyond realism:
# install-files-windows.ps1 registers its service through shawl and does not
# fetch it, while gameapctl puts shawl in C:\gameap\tools\shawl.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$panelDir = if ($env:PANEL_DIR) { $env:PANEL_DIR } else { 'C:\gameap-e2e\panel' }
$panelLog = if ($env:PANEL_LOG) { $env:PANEL_LOG } else { 'C:\gameap-e2e\gameap.log' }
$apiUrl = if ($env:E2E_API_BASE_URL) { $env:E2E_API_BASE_URL } else { 'http://127.0.0.1:8025' }
$nodeWorkPath = if ($env:E2E_NODE_WORK_PATH) { $env:E2E_NODE_WORK_PATH } else { 'C:\gameap' }
$minPanelVersion = [version]'4.5.0'

function Write-Section([string]$Text) { Write-Host "`n=== $Text" }

function Export-Env([string]$Name, [string]$Value) {
  Write-Host "$Name=$Value"
  if ($env:GITHUB_ENV) { "$Name=$Value" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8 }
  Set-Item -Path "env:$Name" -Value $Value
}

function Get-LatestTag([string]$Repo) {
  return (gh release view --repo $Repo --json tagName -q .tagName).Trim()
}

Write-Section 'Excluding the working directories from Defender'
try {
  Add-MpPreference -ExclusionPath 'C:\gameap', 'C:\gameap-e2e' -ErrorAction Stop
} catch {
  Write-Host "::warning::Defender exclusion failed: $($_.Exception.Message)"
}

Write-Section 'Checking that the ports gameap-files needs are free'
$busy = Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
  Where-Object { $_.LocalPort -in 21, 2222, 2121, 2223 }
if ($busy) {
  $ports = ($busy.LocalPort | Sort-Object -Unique) -join ', '
  Write-Host "::error::ports already in use on the runner: $ports"
  exit 1
}

Write-Section 'Checking the upstream sources the installer will use'
foreach ($url in @(
  'https://raw.githubusercontent.com/gameap/scripts/master/ftp/gameap-files/install-files-windows.ps1',
  'https://cdn.gameap.com/gameap-files/releases.json',
  'https://api.github.com/repos/gameap/gameap-files/releases/latest'
)) {
  try {
    Invoke-WebRequest -Uri $url -Method Head -TimeoutSec 15 -UseBasicParsing | Out-Null
  } catch {
    Write-Host "::warning::upstream unreachable, the node install may fail: $url"
  }
}

Write-Section 'Resolving the panel'
New-Item -ItemType Directory -Force -Path $panelDir | Out-Null
if ($env:PANEL_BINARY) {
  Copy-Item -LiteralPath $env:PANEL_BINARY -Destination (Join-Path $panelDir 'gameap.exe') -Force
  $panelTag = if ($env:PANEL_TAG) { $env:PANEL_TAG } else { 'main' }
} else {
  $panelTag = if ($env:PANEL_TAG) { $env:PANEL_TAG } else { Get-LatestTag 'gameap/gameap' }
  $resolved = [version]($panelTag -replace '^v', '' -replace '-.*$', '')
  if ($resolved -lt $minPanelVersion) {
    Write-Host "::error::the plugin needs panel >= $minPanelVersion, resolved $panelTag"
    exit 1
  }

  gh release download --repo gameap/gameap $panelTag --dir $panelDir --pattern 'gameap-*-windows-amd64.zip'
  $zip = Get-ChildItem -Path $panelDir -Filter 'gameap-*-windows-amd64.zip' | Select-Object -First 1
  Expand-Archive -LiteralPath $zip.FullName -DestinationPath $panelDir -Force
}
Export-Env 'PANEL_TAG' $panelTag

Write-Section "Starting the panel ($panelTag)"
New-Item -ItemType Directory -Force -Path $env:FILES_LOCAL_BASE_PATH | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $panelLog) | Out-Null
$panel = Start-Process -FilePath (Join-Path $panelDir 'gameap.exe') -PassThru `
  -RedirectStandardOutput $panelLog -RedirectStandardError "$panelLog.err"
$panel.Id | Out-File -FilePath 'C:\gameap-e2e\gameap.pid' -Encoding ascii

$healthy = $false
foreach ($attempt in 1..90) {
  try {
    Invoke-WebRequest -Uri "$apiUrl/api/health" -TimeoutSec 5 -UseBasicParsing | Out-Null
    $healthy = $true
    break
  } catch {
    Start-Sleep -Seconds 1
  }
}
if (-not $healthy) {
  Write-Host '::error::the panel never became healthy'
  Get-Content -LiteralPath $panelLog -Tail 200 -ErrorAction SilentlyContinue
  Get-Content -LiteralPath "$panelLog.err" -Tail 200 -ErrorAction SilentlyContinue
  exit 1
}

Write-Section 'Installing the daemon with gameapctl'
$gameapctlTag = if ($env:GAMEAPCTL_TAG) { $env:GAMEAPCTL_TAG } else { Get-LatestTag 'gameap/gameapctl' }
Export-Env 'GAMEAPCTL_TAG' $gameapctlTag

$ctlDir = 'C:\gameap-e2e\gameapctl'
Remove-Item -Recurse -Force -LiteralPath $ctlDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $ctlDir | Out-Null
gh release download --repo gameap/gameapctl $gameapctlTag --dir $ctlDir --pattern 'gameapctl-*-windows-amd64.zip'
$ctlZip = Get-ChildItem -Path $ctlDir -Filter 'gameapctl-*-windows-amd64.zip' | Select-Object -First 1
Expand-Archive -LiteralPath $ctlZip.FullName -DestinationPath $ctlDir -Force

$grpcPort = if ($env:GRPC_PORT) { $env:GRPC_PORT } else { '31718' }
& (Join-Path $ctlDir 'gameapctl.exe') --non-interactive daemon install `
  --connect "grpc://127.0.0.1:$grpcPort/$env:DAEMON_SETUP_KEY" `
  --work-path $nodeWorkPath
if ($LASTEXITCODE -ne 0) {
  Write-Host "::error::gameapctl daemon install exited with $LASTEXITCODE"
  exit 1
}

# The gameap-files installer looks here and in PATH, and fetches nothing itself.
if (-not (Test-Path 'C:\gameap\tools\shawl\shawl.exe')) {
  Write-Host '::error::shawl is missing after gameapctl daemon install — a provisioning problem, not a plugin one'
  exit 1
}

Export-Env 'E2E_NODE_HOST' '127.0.0.1'

Write-Section 'Waiting for the node to come online in the panel'
$body = @{ login = $env:E2E_ADMIN_USER; password = $env:E2E_ADMIN_PASSWORD } | ConvertTo-Json
$token = (Invoke-RestMethod -Uri "$apiUrl/api/auth/login" -Method Post -Body $body `
  -ContentType 'application/json' -TimeoutSec 10).token
Write-Host "::add-mask::$token"
$headers = @{ Authorization = "Bearer $token" }

$online = $false
foreach ($attempt in 1..60) {
  try {
    $summary = Invoke-RestMethod -Uri "$apiUrl/api/nodes/summary" -Headers $headers -TimeoutSec 5
    if ($summary.total -ge 1 -and $summary.online -ge 1) { $online = $true; break }
  } catch { }
  Start-Sleep -Seconds 3
}

# $ErrorActionPreference is Stop, so an unreachable panel here would terminate
# the script before the diagnostics below ever printed.
$nodes = $null
try {
  $nodes = Invoke-RestMethod -Uri "$apiUrl/api/nodes" -Headers $headers -TimeoutSec 10
} catch {
  Write-Host "could not read the node list: $($_.Exception.Message)"
}

$windowsNode = $nodes | Where-Object { $_.os -eq 'windows' -and $_.enabled }
if (-not $online -or -not $windowsNode) {
  $detail = if ($null -eq $nodes) { '<no response>' } else { $nodes | ConvertTo-Json -Compress }
  Write-Host "::error::no enabled windows node enrolled: $detail"
  Get-Content -LiteralPath $panelLog -Tail 200 -ErrorAction SilentlyContinue
  Get-Content -LiteralPath 'C:\gameap\daemon\logs\output.log' -Tail 200 -ErrorAction SilentlyContinue
  exit 1
}

Write-Section "Ready: panel $panelTag, node 127.0.0.1 ($nodeWorkPath)"
