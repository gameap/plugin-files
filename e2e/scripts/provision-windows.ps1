# Brings up the Windows leg: the panel and the node both live on the runner.
# Exports E2E_NODE_HOST, PANEL_TAG, PANEL_LOG_DIR and GAMEAPCTL_TAG through
# $env:GITHUB_ENV.
#
# Both the panel and the daemon run as services. That is how they run on a real
# Windows host, and it is also the only way the panel survives: the runner puts
# every step in a job object that kills the whole tree when the step ends, so a
# panel started here as a child process would be gone before the suite runs.
#
# The daemon is installed by gameapctl for one concrete reason beyond realism:
# install-files-windows.ps1 registers its service through shawl and does not
# fetch it, while gameapctl puts shawl in C:\gameap\tools\shawl.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$panelDir = if ($env:PANEL_DIR) { $env:PANEL_DIR } else { 'C:\gameap-e2e\panel' }
$panelLogDir = if ($env:PANEL_LOG_DIR) { $env:PANEL_LOG_DIR } else { 'C:\gameap-e2e\logs' }
$apiUrl = if ($env:E2E_API_BASE_URL) { $env:E2E_API_BASE_URL } else { 'http://127.0.0.1:8025' }
$nodeWorkPath = if ($env:E2E_NODE_WORK_PATH) { $env:E2E_NODE_WORK_PATH } else { 'C:\gameap' }
$minPanelVersion = [version]'4.5.0'
$panelService = 'gameap-e2e-panel'

# Its own copy, deliberately not the one under C:\gameap\tools: that one belongs
# to gameapctl, and the assertion further down has to keep meaning something.
$shawlVersion = 'v1.7.0'
$shawlDir = 'C:\gameap-e2e\shawl'
$shawlExe = Join-Path $shawlDir 'shawl.exe'

# The panel reads its whole configuration from the environment, and a service
# inherits none of the step's, so these are handed to shawl one by one.
$panelEnvNames = @(
  'DATABASE_DRIVER', 'DATABASE_URL', 'AUTH_SECRET', 'ENCRYPTION_KEY',
  'AUTH_REQUIRE_MFA_FOR_ADMINS', 'HTTP_HOST', 'HTTP_PORT', 'FILES_LOCAL_BASE_PATH',
  'ADMIN_LOGIN', 'ADMIN_EMAIL', 'ADMIN_PASSWORD', 'GRPC_PORT', 'GRPC_TLS_ENABLED',
  'GRPC_EXTERNAL_HOST', 'GRPC_EXTERNAL_PORT', 'DAEMON_SETUP_KEY',
  'PLUGINS_PERMISSIONS_ENFORCE'
)

function Write-Section([string]$Text) { Write-Host "`n=== $Text" }

function Export-Env([string]$Name, [string]$Value) {
  Write-Host "$Name=$Value"
  if ($env:GITHUB_ENV) { "$Name=$Value" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8 }
  Set-Item -Path "env:$Name" -Value $Value
}

function Get-LatestTag([string]$Repo) {
  return (gh release view --repo $Repo --json tagName -q .tagName).Trim()
}

function Show-PanelLogs {
  Get-Service -Name $panelService -ErrorAction SilentlyContinue | Format-List
  Get-ChildItem -Path $panelLogDir -Filter '*.log' -ErrorAction SilentlyContinue |
    ForEach-Object { Write-Host "== $($_.FullName)"; Get-Content -LiteralPath $_.FullName -Tail 200 }
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

Write-Section "Registering the panel as a service ($panelTag)"
New-Item -ItemType Directory -Force -Path $env:FILES_LOCAL_BASE_PATH | Out-Null
New-Item -ItemType Directory -Force -Path $panelLogDir, $shawlDir | Out-Null
Export-Env 'PANEL_LOG_DIR' $panelLogDir

# gameap.exe does not speak the Windows service control protocol, so the service
# is a shawl wrapper — the same arrangement gameapctl installs on a real host.
$shawlZip = Join-Path $shawlDir 'shawl.zip'
Invoke-WebRequest -UseBasicParsing -OutFile $shawlZip `
  -Uri "https://github.com/mtkennerly/shawl/releases/download/$shawlVersion/shawl-$shawlVersion-win64.zip"
Expand-Archive -LiteralPath $shawlZip -DestinationPath $shawlDir -Force
if (-not (Test-Path $shawlExe)) {
  Write-Host "::error::shawl $shawlVersion did not unpack to $shawlExe"
  exit 1
}

if (Get-Service -Name $panelService -ErrorAction SilentlyContinue) {
  & sc.exe delete $panelService | Out-Null
  Start-Sleep -Seconds 2
}

$envArgs = @()
foreach ($name in $panelEnvNames) {
  $value = [Environment]::GetEnvironmentVariable($name)
  if ($null -ne $value -and $value -ne '') { $envArgs += @('--env', "$name=$value") }
}

& $shawlExe add --name $panelService --restart `
  --cwd $panelDir --log-dir $panelLogDir --log-as gameap --log-rotate daily `
  @envArgs -- (Join-Path $panelDir 'gameap.exe')
if ($LASTEXITCODE -ne 0) {
  Write-Host "::error::shawl add exited with $LASTEXITCODE"
  exit 1
}

try {
  Start-Service -Name $panelService
} catch {
  Write-Host "::error::could not start ${panelService}: $($_.Exception.Message)"
  Show-PanelLogs
  exit 1
}

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
  Show-PanelLogs
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
  Show-PanelLogs
  Get-Content -LiteralPath 'C:\gameap\daemon\logs\output.log' -Tail 200 -ErrorAction SilentlyContinue
  exit 1
}

Write-Section "Ready: panel $panelTag, node 127.0.0.1 ($nodeWorkPath)"
