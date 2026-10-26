$ErrorActionPreference = 'Stop'
$runtimeRoot = "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application"
$runtime = Get-ChildItem $runtimeRoot -Directory | Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
if (-not $runtime) { throw 'Microsoft Edge WebView2 Runtime is not installed' }
$version = $runtime.Name
$destination = Join-Path $env:RUNNER_TEMP 'framemoor-edge-driver'
New-Item $destination -ItemType Directory -Force | Out-Null
$archive = Join-Path $destination 'driver.zip'
Invoke-WebRequest "https://msedgedriver.microsoft.com/$version/edgedriver_win64.zip" -OutFile $archive
Expand-Archive $archive $destination -Force
$driver = Join-Path $destination 'msedgedriver.exe'
$signature = Get-AuthenticodeSignature $driver
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') { throw 'Invalid Microsoft Edge Driver signature' }
Write-Output "WebView2 $version; $(& $driver --version)"
"FRAMEMOOR_EDGE_DRIVER=$driver" >> $env:GITHUB_ENV
