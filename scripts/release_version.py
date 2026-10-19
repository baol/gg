"""Choose and apply the shared release version before compiling artifacts."""
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib


PATTERN = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"


def parse(value):
    match = re.fullmatch(PATTERN, value)
    if not match:
        raise ValueError(f"Expected MAJOR.MINOR.PATCH, got {value!r}")
    return tuple(map(int, match.groups()))


def choose(floor, tags, same_commit, ref):
    minimum = parse(floor)
    versions = {tag: parse(tag[1:]) for tag in tags if re.fullmatch('v' + PATTERN, tag)}
    if ref.startswith('refs/tags/'):
        tag = ref.removeprefix('refs/tags/')
        if not tag.startswith('v'):
            raise ValueError('Release tags start with v')
        version = parse(tag[1:])
        if version < minimum:
            raise ValueError(f'{tag} is below MIN_VERSION {floor}')
    else:
        existing = [versions[tag] for tag in same_commit if tag in versions]
        if existing:
            version = max(existing)
            if version < minimum:
                raise ValueError('Existing commit version is below MIN_VERSION')
        elif versions:
            major, minor, patch = max(versions.values())
            version = max(minimum, (major, minor, patch + 1))
        else:
            version = minimum
    return '.'.join(map(str, version))


def apply(root, version):
    parse(version)
    manifest = root / 'Cargo.toml'
    lock = root / 'Cargo.lock'
    text = manifest.read_text()
    workspace = tomllib.loads(text)['workspace']
    names = {tomllib.loads((root / member / 'Cargo.toml').read_text())['package']['name']
             for member in workspace['members']}
    text, count = re.subn(r'(\[workspace\.package\]\s*\nversion = ")[^"]+',
                          lambda m: m[1] + version, text, count=1)
    if count != 1:
        raise ValueError('Workspace version field missing')
    lock_text = lock.read_text()
    for name in names:
        pattern = r'(\[\[package\]\]\nname = "' + re.escape(name) + r'"\nversion = ")[^"]+'
        lock_text, count = re.subn(pattern, lambda m: m[1] + version, lock_text)
        if count != 1:
            raise ValueError(f'Expected one local lockfile entry for {name}')
    # Validate both documents before writing either.
    tomllib.loads(text)
    tomllib.loads(lock_text)
    manifest.write_text(text)
    lock.write_text(lock_text)


def git(*args):
    return subprocess.check_output(['git', *args], text=True).splitlines()


def main():
    root = Path(__file__).resolve().parent.parent
    if sys.argv[1:] and sys.argv[1] == 'apply':
        apply(root, sys.argv[2])
        return
    floor = (root / 'MIN_VERSION').read_text().strip()
    version = choose(floor, git('tag', '--list'), git('tag', '--points-at', 'HEAD'),
                     os.environ.get('GITHUB_REF', ''))
    print(version)
    if output := os.environ.get('GITHUB_OUTPUT'):
        with open(output, 'a') as file:
            file.write(f'version={version}\n')


if __name__ == '__main__':
    main()
