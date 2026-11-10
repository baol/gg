"""install.sh downloads the archive for this machine, checks its SHA-256 and
installs git-good; a download that does not match installs nothing."""
import hashlib
import http.server
import io
import os
import platform
import subprocess
import tarfile
import tempfile
import threading
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("install.sh")


def target():
    system = {"Linux": "unknown-linux-gnu", "Darwin": "apple-darwin"}[platform.system()]
    machine = platform.machine().lower()
    arch = "aarch64" if machine in ("arm64", "aarch64") else "x86_64"
    if system == "apple-darwin" and arch == "x86_64":
        arm = subprocess.run(["sysctl", "-n", "hw.optional.arm64"],
                             capture_output=True, text=True).stdout.strip()
        if arm == "1":
            arch = "aarch64"
    return f"git-good-{arch}-{system}.tar.gz"


def archive(version):
    """A release archive whose git-good prints its version, like the real one."""
    data = io.BytesIO()
    script = f"#!/bin/sh\necho 'git-good {version}'\n".encode()
    with tarfile.open(fileobj=data, mode="w:gz") as tar:
        for name, body in (("./git-good", script), ("./README.md", b"readme\n")):
            info = tarfile.TarInfo(name)
            info.size = len(body)
            info.mode = 0o755
            tar.addfile(info, io.BytesIO(body))
    return data.getvalue()


@unittest.skipIf(platform.system() not in ("Linux", "Darwin"), "install.sh is for Linux and macOS")
class InstallScript(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.served = self.root / "release"
        self.served.mkdir()
        served = str(self.served)

        class Quiet(http.server.SimpleHTTPRequestHandler):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, directory=served, **kwargs)

            def log_message(self, *args):
                pass

        handler = Quiet
        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.addCleanup(self.server.shutdown)

    def publish(self, body, digest=None):
        name = target()
        (self.served / name).write_bytes(body)
        digest = digest or hashlib.sha256(body).hexdigest()
        (self.served / f"{name}.sha256").write_text(f"{digest}  {name}\n")

    def install(self, path="/usr/bin:/bin"):
        dest = self.root / "bin"
        env = {
            "HOME": str(self.root),
            "PATH": path,
            "GG_INSTALL_DIR": str(dest),
            "GG_DOWNLOAD_URL": f"http://127.0.0.1:{self.server.server_port}/",
        }
        result = subprocess.run(["sh", str(SCRIPT)], env=env, capture_output=True, text=True)
        return result, dest / "git-good"

    def test_installs_the_checked_archive_and_says_how_to_run_it(self):
        self.publish(archive("9.9.9"))
        result, binary = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Installed git-good 9.9.9", result.stdout)
        self.assertTrue(os.access(binary, os.X_OK))
        self.assertIn("Add", result.stdout, "not on PATH yet")
        # A second run replaces the binary, and on PATH it says how to run it.
        self.publish(archive("9.9.10"))
        result, binary = self.install(f"{binary.parent}:/usr/bin:/bin")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("git-good 9.9.10", result.stdout)
        self.assertIn("Run it with: git good", result.stdout)

    def test_a_download_that_does_not_match_its_checksum_installs_nothing(self):
        self.publish(archive("9.9.9"), digest="0" * 64)
        result, binary = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match its checksum", result.stderr)
        self.assertFalse(binary.exists())

    def test_the_script_is_plain_ascii(self):
        # macOS /bin/sh reads a non-ASCII character after `$name` as part
        # of the name, which `set -u` then rejects.
        SCRIPT.read_bytes().decode("ascii")

    def test_a_missing_release_fails_with_its_address(self):
        result, binary = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("could not download", result.stderr)
        self.assertFalse(binary.exists())


if __name__ == "__main__":
    unittest.main()
