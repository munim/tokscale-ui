$ErrorActionPreference = "Stop"

$Repo = if ($env:TOKSCALE_UI_REPO) { $env:TOKSCALE_UI_REPO } else { "munim/tokscale-ui" }
$BaseUrl = "https://github.com/$Repo/releases/latest/download"
$ManifestUrl = "$BaseUrl/latest.json"
$SumsUrl = "$BaseUrl/SHA256SUMS.txt"

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("tokscale-ui-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir | Out-Null

try {
    Write-Host "Fetching release manifest..."
    $Manifest = Invoke-RestMethod -Uri $ManifestUrl
    $WinEntry = $Manifest.platforms."windows-x86_64"
    if (-not $WinEntry -or -not $WinEntry.url) {
        throw "Could not resolve windows-x86_64 download URL from $ManifestUrl"
    }

    $AssetUrl = $WinEntry.url
    $AssetName = [System.IO.Path]::GetFileName(($AssetUrl -split '\?')[0])
    $InstallerPath = Join-Path $TempDir $AssetName

    Write-Host "Downloading $AssetName..."
    Invoke-WebRequest -Uri $AssetUrl -OutFile $InstallerPath -UseBasicParsing

    try {
        $SumsText = Invoke-RestMethod -Uri $SumsUrl
        $Line = ($SumsText -split "`n") | Where-Object { $_ -match [regex]::Escape($AssetName) } | Select-Object -First 1
        if ($Line) {
            $Expected = ($Line.Trim() -split "\s+")[0].ToLowerInvariant()
            $Actual = (Get-FileHash -Path $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($Expected -ne $Actual) {
                throw "SHA-256 mismatch for $AssetName (expected $Expected, got $Actual)"
            }
            Write-Host "Verified SHA-256 checksum."
        }
    } catch {
        if ($_.Exception.Message -like "SHA-256 mismatch*") {
            throw
        }
    }

    Write-Host "Installing Tokscale UI silently (per-user)..."
    $Proc = Start-Process -FilePath $InstallerPath -ArgumentList "/S" -Wait -PassThru
    if ($Proc.ExitCode -ne 0) {
        throw "Installer exited with code $($Proc.ExitCode)"
    }

    Write-Host "Tokscale UI installed successfully."
} finally {
    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
