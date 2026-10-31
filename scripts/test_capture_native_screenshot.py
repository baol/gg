#!/usr/bin/env python3
import contextlib
import io
import subprocess
import tempfile
import unittest
from pathlib import Path

import capture_native_screenshot as capture
from ppm_to_png import ppm_to_png

PIXEL = b"P6\n1 1\n255\n\x01\x02\x03"


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *arguments],
        cwd=root,
        check=True,
        capture_output=True,
    )


class FingerprintTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        git(self.root, "init", "-q")
        self.write("apps/git-good/src/main.rs", "fn main() {}\n")
        self.write("README.md", "gg\n")

    def tearDown(self) -> None:
        self.directory.cleanup()

    def write(self, name: str, text: str) -> None:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def stamp(self, value: str | None) -> None:
        text = {} if value is None else {capture.STAMP: value}
        path = self.root / capture.SCREENSHOT
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(ppm_to_png(PIXEL, text))

    def commit(self) -> None:
        git(self.root, "add", "--all")
        git(self.root, "commit", "-q", "-m", "change")

    def check(self) -> int:
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
            io.StringIO()
        ):
            return capture.check(self.root)

    def test_a_capture_stamped_from_the_index_matches_the_commit(self) -> None:
        git(self.root, "add", "--all")
        self.stamp(capture.sources_fingerprint(self.root))
        self.commit()
        self.assertEqual(
            capture.sources_fingerprint(self.root),
            capture.sources_fingerprint(self.root, "HEAD"),
        )
        self.assertEqual(self.check(), 0)

    def test_a_ui_change_without_a_new_capture_fails(self) -> None:
        git(self.root, "add", "--all")
        self.stamp(capture.sources_fingerprint(self.root))
        self.commit()
        self.write("apps/git-good/src/main.rs", "fn main() { todo!() }\n")
        self.commit()
        self.assertEqual(self.check(), 1)

    def test_other_files_do_not_change_the_fingerprint(self) -> None:
        git(self.root, "add", "--all")
        before = capture.sources_fingerprint(self.root)
        self.write("README.md", "git good\n")
        git(self.root, "add", "--all")
        self.assertEqual(capture.sources_fingerprint(self.root), before)

    def test_an_unstamped_screenshot_fails(self) -> None:
        self.stamp(None)
        self.commit()
        self.assertEqual(self.check(), 1)


if __name__ == "__main__":
    unittest.main()
