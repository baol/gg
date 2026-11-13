#!/usr/bin/env python3
"""Wrap the release binary in Git Good.app and a .dmg to drag to Applications.

Usage: package_macos_app.py BINARY VERSION ICON_PNG OUT_DIR

Runs on the macOS release job after the binary is built, so nothing is
compiled again: it copies the binary into the bundle, turns the PNG into an
.icns with sips and iconutil, signs the bundle ad hoc, and packs a compressed
disk image with hdiutil. Writes OUT_DIR/git-good-aarch64-apple-darwin.dmg and
its .sha256.
"""
import hashlib
import plistlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

APP = "Git Good.app"
IDENTIFIER = "io.github.baol.git-good"
DMG = "git-good-aarch64-apple-darwin.dmg"


def info_plist(version):
    """The bundle's Info.plist: what Finder, the Dock and Launch Services read."""
    return {
        "CFBundleName": "Git Good",
        "CFBundleDisplayName": "Git Good",
        "CFBundleIdentifier": IDENTIFIER,
        "CFBundleExecutable": "git-good",
        "CFBundleIconFile": "AppIcon",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "CFBundleInfoDictionaryVersion": "6.0",
        "LSMinimumSystemVersion": "11.0",
        "LSApplicationCategoryType": "public.app-category.developer-tools",
        "NSHighResolutionCapable": True,
    }


def bundle(binary, version, icon, dest, make_icns):
    """Lay out dest/Git Good.app; `make_icns(png, icns)` writes the icon."""
    app = Path(dest) / APP
    macos = app / "Contents" / "MacOS"
    resources = app / "Contents" / "Resources"
    macos.mkdir(parents=True)
    resources.mkdir()
    shutil.copy2(binary, macos / "git-good")
    (macos / "git-good").chmod(0o755)
    with open(app / "Contents" / "Info.plist", "wb") as out:
        plistlib.dump(info_plist(version), out)
    make_icns(Path(icon), resources / "AppIcon.icns")
    return app


def icns(png, out):
    """sips scales the PNG to each icon size; iconutil packs them."""
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
                pixels = str(size * scale)
                run("sips", "-z", pixels, pixels, str(png), "--out", str(iconset / name))
        run("iconutil", "-c", "icns", str(iconset), "-o", str(out))


def run(*command):
    subprocess.run(command, check=True, stdout=subprocess.DEVNULL)


def main(binary, version, icon, out_dir):
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        staging = Path(tmp) / "Git Good"
        staging.mkdir()
        app = bundle(binary, version, icon, staging, icns)
        # Ad hoc: macOS needs a signature on Apple silicon; there is no
        # Developer ID, so Gatekeeper still asks on first open.
        run("codesign", "--force", "--deep", "--sign", "-", str(app))
        (staging / "Applications").symlink_to("/Applications")
        dmg = out / DMG
        dmg.unlink(missing_ok=True)
        run("hdiutil", "create", "-volname", "Git Good", "-srcfolder", str(staging),
            "-ov", "-format", "UDZO", str(dmg))
    digest = hashlib.sha256(dmg.read_bytes()).hexdigest()
    Path(str(dmg) + ".sha256").write_text(f"{digest}  {dmg.name}\n")
    print(f"wrote {dmg}")


if __name__ == "__main__":
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    main(*sys.argv[1:])
