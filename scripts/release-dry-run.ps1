# Dry-run of the release workflow's Windows packaging step.
#
# Mirrors the "Package (windows)" step in .github/workflows/release.yml using
# local values, so the PowerShell syntax is proven before it runs on a runner.

$ErrorActionPreference = 'Stop'

# Same values the workflow injects from needs.tag.outputs.version / matrix.bin
$version = (Select-String -Path Cargo.toml -Pattern '^version' | Select-Object -First 1).Line.Split('"')[1]
$name = "bpmc-$version-windows-x86_64"
$bin = "target/release/bpmc.exe"

Write-Host "== version: $version"
Write-Host "== name: $name"
Write-Host "== bin: $bin"

Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "dist/$name" | Out-Null
Copy-Item $bin "dist/$name/"
Copy-Item README.md, LICENSE "dist/$name/"
Compress-Archive -Path "dist/$name" -DestinationPath "dist/$name.zip" -Force
Remove-Item -Recurse -Force "dist/$name"
$hash = (Get-FileHash "dist/$name.zip" -Algorithm SHA256).Hash.ToLower()
"$hash  $name.zip" | Out-File -Encoding ascii "dist/$name.zip.sha256"
Get-ChildItem dist

Write-Host "== checksum content"
Get-Content "dist/$name.zip.sha256"

Write-Host "== zip listing"
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path "dist/$name.zip"))
$zip.Entries | ForEach-Object { $_.FullName }
$zip.Dispose()

Write-Host "== gh CLI available locally?"
if (Get-Command gh -ErrorAction SilentlyContinue) { Write-Host "gh: yes" } else { Write-Host "gh: no (runner images ship it; local machine does not)" }
