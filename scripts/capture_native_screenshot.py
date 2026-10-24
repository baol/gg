#!/usr/bin/env python3
"""Regenerate the README's native screenshot from the current UI sources."""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from ppm_to_png import ppm_to_png

SCREENSHOT = "docs/screenshots/gg-native-wide.png"
WINDOW_SIZE = "1120x740"
RELEVANT = (
    "apps/git-good/src",
    "apps/git-good/assets",
    "apps/git-good/Cargo.toml",
    "Cargo.toml",
    "Cargo.lock",
)
IN_PROGRESS = (
    "rebase-merge",
    "rebase-apply",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
)


def repo_root() -> Path:
    output = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    )
    return Path(output.strip())


def _names(root: Path, *arguments: str) -> list[str]:
    output = subprocess.check_output(
        ["git", *arguments, "--name-only", "--", *RELEVANT],
        cwd=root,
        text=True,
    )
    return [line for line in output.splitlines() if line]


def staged_ui(root: Path) -> list[str]:
    return _names(root, "diff", "--cached", "--diff-filter=ACMR")


def unstaged_ui(root: Path) -> list[str]:
    return _names(root, "diff")


def untracked_ui(root: Path) -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "--", *RELEVANT],
        cwd=root,
        text=True,
    )
    return [line for line in output.splitlines() if line]


def operation_in_progress(root: Path) -> str | None:
    git_dir = Path(
        subprocess.check_output(
            ["git", "rev-parse", "--git-dir"], cwd=root, text=True
        ).strip()
    )
    if not git_dir.is_absolute():
        git_dir = root / git_dir
    for name in IN_PROGRESS:
        if (git_dir / name).exists():
            return name
    return None


def build(root: Path) -> Path:
    subprocess.run(
        ["cargo", "build", "-p", "git-good", "--locked"], cwd=root, check=True
    )
    name = "git-good.exe" if os.name == "nt" else "git-good"
    binary = root / "target" / "debug" / name
    if not binary.is_file():
        raise SystemExit(f"screenshot: missing built binary at {binary}")
    return binary


def capture(root: Path, binary: Path) -> Path:
    with tempfile.TemporaryDirectory(prefix="gg-screenshot-") as directory:
        ppm = Path(directory) / "gg.ppm"
        environment = os.environ.copy()
        environment.pop("WAYLAND_DISPLAY", None)
        environment.update(
            {
                "GG_OFFLINE": "1",
                "GG_SCREENSHOT": str(ppm),
                "GG_WINDOW_SIZE": WINDOW_SIZE,
            }
        )
        command = [str(binary), str(root)]
        if sys.platform.startswith("linux"):
            xvfb = shutil.which("xvfb-run")
            if xvfb:
                command = [
                    xvfb,
                    "-a",
                    "-s",
                    "-screen 0 1280x900x24",
                    *command,
                ]
            elif not environment.get("DISPLAY"):
                raise SystemExit(
                    "screenshot: install xvfb-run or set DISPLAY to capture natively"
                )
        subprocess.run(
            command,
            cwd=root,
            env=environment,
            check=True,
            timeout=180,
        )
        if not ppm.is_file():
            raise SystemExit("screenshot: the app did not write a capture")
        target = root / SCREENSHOT
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(ppm_to_png(ppm.read_bytes()))
        return target


def _report(paths: list[str]) -> None:
    for path in paths[:10]:
        print(f"  {path}", file=sys.stderr)
    if len(paths) > 10:
        print(f"  … and {len(paths) - 10} more", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--force",
        action="store_true",
        help="capture even when no UI files are staged",
    )
    arguments = parser.parse_args(argv)

    if os.environ.get("GG_SKIP_SCREENSHOT") or os.environ.get("CI"):
        print("screenshot: skipped by environment", file=sys.stderr)
        return 0
    root = repo_root()
    if in_progress := operation_in_progress(root):
        print(
            f"screenshot: skipped while a Git operation is in progress ({in_progress})",
            file=sys.stderr,
        )
        return 0
    if not arguments.force:
        if not staged_ui(root):
            return 0
        dirty = unstaged_ui(root)
        if dirty:
            print(
                "screenshot: staged UI files also have unstaged edits:",
                file=sys.stderr,
            )
            _report(dirty)
            print(
                "screenshot: stage or stash them, or set GG_SKIP_SCREENSHOT=1",
                file=sys.stderr,
            )
            return 1
        untracked = untracked_ui(root)
        if untracked:
            print("screenshot: untracked UI files are not part of the commit:", file=sys.stderr)
            _report(untracked)
            print(
                "screenshot: add them, or set GG_SKIP_SCREENSHOT=1",
                file=sys.stderr,
            )
            return 1

    binary = build(root)
    target = capture(root, binary)
    subprocess.run(["git", "add", "--", SCREENSHOT], cwd=root, check=True)
    print(f"screenshot: updated {target.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
