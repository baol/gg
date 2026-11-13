import plistlib
import tempfile
import unittest
from pathlib import Path

import package_macos_app as package


class Bundle(unittest.TestCase):
    def test_the_app_holds_the_binary_its_info_and_its_icon(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = root / "git-good"
            binary.write_bytes(b"#!/bin/sh\n")
            icon = root / "gg.png"
            icon.write_bytes(b"png")
            made = []
            app = package.bundle(binary, "0.1.100", icon, root / "stage",
                                 lambda png, icns: (made.append((png, icns)), icns.write_bytes(b"icns")))
            self.assertEqual(app.name, "Git Good.app")
            executable = app / "Contents" / "MacOS" / "git-good"
            self.assertEqual(executable.read_bytes(), b"#!/bin/sh\n")
            self.assertTrue(executable.stat().st_mode & 0o111, "executable")
            info = plistlib.loads((app / "Contents" / "Info.plist").read_bytes())
            self.assertEqual(info["CFBundleExecutable"], "git-good")
            self.assertEqual(info["CFBundleIconFile"], "AppIcon")
            self.assertEqual(info["CFBundleShortVersionString"], "0.1.100")
            self.assertEqual(info["CFBundlePackageType"], "APPL")
            self.assertEqual(made, [(icon, app / "Contents" / "Resources" / "AppIcon.icns")])


if __name__ == "__main__":
    unittest.main()
