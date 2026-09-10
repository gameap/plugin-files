# Best-effort diagnostics for a failed Windows leg. Never fails the job, and
# never copies out the database or the raw users.d files: both carry password
# hashes.
param([string]$Out = 'e2e-logs')

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

$nodeWorkPath = if ($env:E2E_NODE_WORK_PATH) { $env:E2E_NODE_WORK_PATH } else { 'C:\gameap' }
$panelLog = if ($env:PANEL_LOG) { $env:PANEL_LOG } else { 'C:\gameap-e2e\gameap.log' }
$apiUrl = if ($env:E2E_API_BASE_URL) { $env:E2E_API_BASE_URL } else { 'http://127.0.0.1:8025' }

New-Item -ItemType Directory -Force -Path $Out | Out-Null

# The local preference makes a missing path a terminating error, so the reason
# lands inside the artifact instead of as red noise in the job log.
function Save([string]$Name, [scriptblock]$Body) {
  $path = Join-Path $Out $Name
  try {
    $ErrorActionPreference = 'Stop'
    & $Body | Out-File -FilePath $path -Encoding utf8
  } catch {
    $_.Exception.Message | Out-File -FilePath $path -Encoding utf8
  }
}

Copy-Item -LiteralPath $panelLog -Destination (Join-Path $Out 'panel.log') -ErrorAction SilentlyContinue
Copy-Item -LiteralPath "$panelLog.err" -Destination (Join-Path $Out 'panel.err.log') -ErrorAction SilentlyContinue

Save 'panel-plugins-dir.txt' { Get-ChildItem -Path (Join-Path $env:FILES_LOCAL_BASE_PATH 'plugins') -Force }
Save 'services.txt' { Get-Service -Name 'gameap-files', 'gameap-daemon', 'GameAP Daemon' -ErrorAction SilentlyContinue | Format-List }
Save 'sc-qc-gameap-files.txt' { & sc.exe qc gameap-files }
Save 'listening-ports.txt' { Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Sort-Object LocalPort | Format-Table -AutoSize }
Save 'defender.txt' { Get-MpComputerStatus }
Save 'gameap-files-config.yaml' { Get-Content -LiteralPath (Join-Path $nodeWorkPath '.plugins\files\config.yaml') -Raw }
# The listing only, never the contents: a drop-in holds an argon2 hash.
Save 'users-d-listing.txt' { Get-ChildItem -LiteralPath (Join-Path $nodeWorkPath '.plugins\files\users.d') -Force }
Save 'gameap-files-service-logs.txt' { Get-ChildItem -Path 'C:\gameap\services\logs\gameap-files' -Recurse -ErrorAction SilentlyContinue | ForEach-Object { "== $($_.FullName)"; Get-Content -LiteralPath $_.FullName -Tail 400 } }
Save 'daemon-output.log' { Get-Content -LiteralPath 'C:\gameap\daemon\logs\output.log' -Tail 400 }
# Get-EventLog exists only in Windows PowerShell, and the workflow runs pwsh.
Save 'eventlog.txt' {
  Get-WinEvent -LogName Application -MaxEvents 200 -ErrorAction SilentlyContinue |
    Where-Object { $_.Message -match 'gameap|shawl' } | Format-List
}

try {
  $body = @{ login = $env:E2E_ADMIN_USER; password = $env:E2E_ADMIN_PASSWORD } | ConvertTo-Json
  $token = (Invoke-RestMethod -Uri "$apiUrl/api/auth/login" -Method Post -Body $body `
    -ContentType 'application/json' -TimeoutSec 10).token
  Write-Host "::add-mask::$token"
  Invoke-RestMethod -Uri "$apiUrl/api/admin/plugins/loaded" -Headers @{ Authorization = "Bearer $token" } |
    ConvertTo-Json -Depth 6 | Out-File -FilePath (Join-Path $Out 'plugins-loaded.json') -Encoding utf8
} catch {
  Write-Host "could not read the loaded plugins: $($_.Exception.Message)"
}

Write-Host "collected into ${Out}:"
Get-ChildItem -Path $Out | Format-Table -AutoSize

# Diagnostics must never fail the job. sc.exe above leaves its exit code in
# $LASTEXITCODE, which the pwsh step would otherwise exit with.
exit 0
