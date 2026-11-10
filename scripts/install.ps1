# Install Git Good on Windows (PowerShell 5.1 or later):
#
#   irm https://raw.githubusercontent.com/baol/gg/main/scripts/install.ps1 | iex
#
# It downloads the release archive for this machine from GitHub Releases,
# checks it against the published SHA-256, puts git-good.exe in
# %LOCALAPPDATA%\Programs\git-good and adds that folder to your user PATH, so
# `git good` runs it.
#
# Settings (environment variables):
#   GG_VERSION      a release such as 0.1.90 (default: the latest)
#   GG_INSTALL_DIR  where to put git-good.exe
#   GG_DOWNLOAD_URL a folder holding the archives, instead of GitHub
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# Stop with an error rather than `exit`, which would close the window that
# ran `irm | iex`.
function Fail($message) {
    throw "git-good install: $message"
}

$repo = 'baol/gg'
$dir = if ($env:GG_INSTALL_DIR) { $env:GG_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\git-good' }
$version = if ($env:GG_VERSION) { $env:GG_VERSION.TrimStart('v') } else { 'latest' }

$machine = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$arch = switch ($machine) {
    'X64' { 'x86_64' }
    'Arm64' { 'aarch64' }
    default { Fail "unsupported processor $machine" }
}
$name = "git-good-$arch-pc-windows-msvc.zip"

$base = if ($env:GG_DOWNLOAD_URL) {
    $env:GG_DOWNLOAD_URL.TrimEnd('/')
} elseif ($version -eq 'latest') {
    "https://github.com/$repo/releases/latest/download"
} else {
    "https://github.com/$repo/releases/download/v$version"
}

[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$work = Join-Path ([IO.Path]::GetTempPath()) ("git-good-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
    Write-Host "Downloading $name..."
    $archive = Join-Path $work $name
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$name" -OutFile $archive
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$name.sha256" -OutFile "$archive.sha256"
    } catch {
        Fail "could not download $base/$name ($($_.Exception.Message))"
    }
    $expected = ((Get-Content -Raw "$archive.sha256").Trim() -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    if (-not $expected -or $expected -ne $actual) {
        Fail "the download does not match its checksum (expected $expected, got $actual); nothing was installed"
    }

    $unpacked = Join-Path $work 'archive'
    Expand-Archive -Path $archive -DestinationPath $unpacked
    $binary = Join-Path $unpacked 'git-good.exe'
    if (-not (Test-Path $binary)) { Fail 'the archive holds no git-good.exe' }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $target = Join-Path $dir 'git-good.exe'
    Copy-Item -Force $binary $target
    Unblock-File $target

    $installed = & $target --version
    if ($LASTEXITCODE -ne 0) { Fail "$target does not run" }
    Write-Host "Installed $installed to $target."

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $parts = @($userPath -split ';' | Where-Object { $_ })
    if ($parts -notcontains $dir) {
        [Environment]::SetEnvironmentVariable('Path', (($parts + $dir) -join ';'), 'User')
        Write-Host "Added $dir to your user PATH; open a new terminal, then run: git good"
    } else {
        Write-Host 'Run it with: git good'
    }
    if (($env:Path -split ';') -notcontains $dir) { $env:Path = "$env:Path;$dir" }
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
