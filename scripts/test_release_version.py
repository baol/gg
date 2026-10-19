"""Release numbering and manifest consistency regressions."""
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest

from release_version import apply, choose, parse


class VersionTests(unittest.TestCase):
    def test_first_release_uses_floor(self):
        self.assertEqual(choose('0.2.0', ['commit-abc'], [], ''), '0.2.0')

    def test_patch_increment_is_numeric(self):
        self.assertEqual(choose('0.1.0', ['v0.1.9', 'v0.1.10', 'v0.1.2'], [], ''), '0.1.11')

    def test_floor_starts_new_series(self):
        self.assertEqual(choose('1.0.0', ['v0.9.9'], [], ''), '1.0.0')
        self.assertEqual(choose('0.2.0', ['v0.2.0'], [], ''), '0.2.1')

    def test_retry_reuses_commits_version(self):
        self.assertEqual(choose('0.1.0', ['v0.1.1', 'v0.1.2'], ['v0.1.1'], ''), '0.1.1')

    def test_explicit_tag_and_floor(self):
        self.assertEqual(choose('0.2.0', [], [], 'refs/tags/v0.2.3'), '0.2.3')
        with self.assertRaises(ValueError):
            choose('0.2.0', [], [], 'refs/tags/v0.1.9')
        with self.assertRaises(ValueError):
            choose('0.2.0', ['v0.1.0'], ['v0.1.0'], '')

    def test_invalid_versions(self):
        for version in ['1.2', '01.2.3', '1.2.-3', 'v1.2.3', '1.2.3;echo bad']:
            with self.assertRaises(ValueError):
                parse(version)

    def test_apply_updates_local_packages_and_preserves_dependencies(self):
        source = Path(__file__).resolve().parent.parent
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ['Cargo.toml', 'Cargo.lock', 'apps/git-good/Cargo.toml',
                         'crates/engine/Cargo.toml', 'crates/git/Cargo.toml']:
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source / name, target)
            before = tomllib.loads((root / 'Cargo.lock').read_text())
            apply(root, '0.3.7')
            self.assertEqual(tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version'], '0.3.7')
            after = tomllib.loads((root / 'Cargo.lock').read_text())
            local = {'gg-engine', 'gg-git', 'git-good'}
            for old, new in zip(before['package'], after['package'], strict=True):
                if old['name'] in local:
                    self.assertEqual(new['version'], '0.3.7')
                    old = {**old, 'version': '0.3.7'}
                self.assertEqual(old, new)
            first = (root / 'Cargo.lock').read_bytes()
            apply(root, '0.3.7')
            self.assertEqual(first, (root / 'Cargo.lock').read_bytes())


if __name__ == '__main__':
    unittest.main()
