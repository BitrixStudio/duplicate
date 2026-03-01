$ErrorActionPreference = "Stop"

$Repo = "BitrixStudio/duplicate"
$Bin  = "duplicate"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE "bin" }

# Detect arch
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { throw "32-bit Windows not supported" }
$Target = "$Arch-pc-windows-msvc"

Write-Host "Installing $Bin for $Target..."

# Latest tag
$latest = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$tag = $latest.tag_name

$asset = "$Bin-$tag-$Target.zip"
$url = "https://github.com/$Repo/releases/download/$tag/$asset"

$tmp = New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString())
$zipPath = Join-Path $tmp $asset

Invoke-WebRequest -Uri $url -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $tmp -Force

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item -Force (Join-Path $tmp "$Bin.exe") (Join-Path $InstallDir "$Bin.exe")

Write-Host "Installed to: $InstallDir\$Bin.exe"

$path = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($path.Split(";") -contains $InstallDir)) {
  Write-Host ""
  Write-Host "Adding $InstallDir to your user PATH..."
  [Environment]::SetEnvironmentVariable("Path", "$path;$InstallDir", "User")
  Write-Host "Restart your terminal for PATH changes to take effect."
}

Write-Host "Run: $Bin --help"