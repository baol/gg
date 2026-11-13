#!/bin/sh
# Install Git Good on Linux or macOS:
#
#   curl -fsSL https://raw.githubusercontent.com/baol/gg/main/scripts/install.sh | sh
#
# It downloads the release archive for this machine from GitHub Releases,
# checks it against the published SHA-256, and puts `git-good` in
# ~/.local/bin, so `git good` runs it.
#
# Settings (environment variables):
#   GG_VERSION      a release such as 0.1.90 (default: the latest)
#   GG_INSTALL_DIR  where to put git-good (default: ~/.local/bin)
#   GG_DOWNLOAD_URL a folder holding the archives, instead of GitHub
set -eu

repo="baol/gg"
dir="${GG_INSTALL_DIR:-$HOME/.local/bin}"
version="${GG_VERSION:-latest}"

fail() {
    echo "git-good install: $*" >&2
    exit 1
}

case "$(uname -s)" in
    Linux) system="unknown-linux-gnu" ;;
    Darwin) system="apple-darwin" ;;
    *) fail "unsupported system $(uname -s); on Windows use install.ps1" ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) fail "unsupported processor $(uname -m)" ;;
esac
# A shell running under Rosetta reports x86_64 on an Apple silicon Mac.
if [ "$system" = apple-darwin ] && [ "$arch" = x86_64 ] &&
    [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
    arch="aarch64"
fi
# Releases have no Intel Mac build; Cargo builds one from source.
if [ "$system" = apple-darwin ] && [ "$arch" = x86_64 ]; then
    fail "there is no release build for Intel Macs; install it with: cargo install git-good --locked"
fi
name="git-good-$arch-$system.tar.gz"

if [ -n "${GG_DOWNLOAD_URL:-}" ]; then
    base="${GG_DOWNLOAD_URL%/}"
elif [ "$version" = latest ]; then
    base="https://github.com/$repo/releases/latest/download"
else
    base="https://github.com/$repo/releases/download/v${version#v}"
fi

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --proto '=https,http' -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        fail "needs curl or wget"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        fail "needs sha256sum or shasum to check the download"
    fi
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Downloading ${name}..."
download "$base/$name" "$work/$name" || fail "could not download $base/$name"
download "$base/$name.sha256" "$work/$name.sha256" ||
    fail "could not download the checksum $base/$name.sha256"
expected="$(cut -d ' ' -f 1 <"$work/$name.sha256")"
actual="$(sha256 "$work/$name")"
[ -n "$expected" ] && [ "$expected" = "$actual" ] ||
    fail "the download does not match its checksum (expected $expected, got $actual); nothing was installed"

mkdir -p "$work/archive"
# As root, keep the files ours rather than the build machine's user.
tar -xzf "$work/$name" -C "$work/archive" --no-same-owner
[ -f "$work/archive/git-good" ] || fail "the archive holds no git-good"
chmod 755 "$work/archive/git-good"
mkdir -p "$dir"
# Replace any older copy in one step, even while it runs.
mv -f "$work/archive/git-good" "$dir/git-good"

installed="$("$dir/git-good" --version)" || fail "$dir/git-good does not run"
echo "Installed $installed to $dir/git-good."
case ":$PATH:" in
    *":$dir:"*) echo "Run it with: git good" ;;
    *)
        echo "Add $dir to your PATH to run it as \`git good\`, for example:"
        echo "  echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile"
        ;;
esac
