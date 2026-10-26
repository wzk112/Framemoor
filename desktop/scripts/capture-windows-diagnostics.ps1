param([Parameter(Mandatory=$true)][string]$Results)
# Runs only on the disposable Windows test runner after a native test failure.
$ErrorActionPreference = 'Stop'
Get-Process framemoor-desktop,msedgewebview2 -ErrorAction SilentlyContinue |
  Select-Object Id,ProcessName,MainWindowTitle,Responding |
  ConvertTo-Json | Set-Content (Join-Path $Results 'native-processes.json') -Encoding UTF8
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width,$bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
  $graphics.CopyFromScreen($bounds.Location,[System.Drawing.Point]::Empty,$bounds.Size)
  $bitmap.Save((Join-Path $Results 'native-desktop.png'),[System.Drawing.Imaging.ImageFormat]::Png)
} finally {
  $graphics.Dispose()
  $bitmap.Dispose()
}
