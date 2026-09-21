# Open the locally running Work dashboard on the Windows office monitor.
$ErrorActionPreference = 'Stop'
$dashboardUrl = 'http://localhost:8789'
$response = Invoke-RestMethod -Uri "$dashboardUrl/api/state" -TimeoutSec 10
if ($response.profile -ne 'work') { throw 'Refusing to open a non-Work dashboard on the office display.' }
$edgePaths = @(
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
)
$edgePath = $edgePaths | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edgePath) { throw 'Microsoft Edge was not found.' }
Start-Process -FilePath $edgePath -ArgumentList @('--kiosk', $dashboardUrl, '--edge-kiosk-type=fullscreen', '--no-first-run')
