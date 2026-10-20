"""Create a packed, deterministic 10,000-file review benchmark repository."""
import pathlib
import os
import subprocess
import sys

# Fixture contents and packing are independent of personal Git settings.
os.environ.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
repo = pathlib.Path(sys.argv[1]).resolve()
repo.mkdir()  # Keep existing repositories intact.
subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
stream = bytearray()
for mark, data in [(1, b"before\n"), (2, b"after\n")]:
    stream.extend(f"blob\nmark :{mark}\ndata {len(data)}\n".encode() + data + b"\n")
# Exactly 1 MiB on each side, with changes distributed across the file.
before = b"".join(f"{i:06d} ".encode() + b"x" * 56 + b"\n" for i in range(16384))
after = before.replace(b"000000 x", b"000000 y").replace(b"016383 x", b"016383 y")
assert len(before) == len(after) == 1024 * 1024
for mark, data in [(3, before), (4, after)]:
    stream.extend(f"blob\nmark :{mark}\ndata {len(data)}\n".encode() + data + b"\n")
for version in range(2):
    message = f"Review benchmark {version}"
    stream.extend(f"commit refs/heads/main\ncommitter Benchmark <benchmark@example.com> {1700000000 + version} +0000\ndata {len(message)}\n{message}\n".encode())
    stream.extend(f"M 100644 :{3 + version} 00000.txt\n".encode())
    for index in range(1, 10000):
        stream.extend(f"M 100644 :{1 + version} {index:05d}.txt\n".encode())
    stream.extend(b"\n")
subprocess.run(["git", "-C", str(repo), "fast-import", "--quiet"], input=stream, check=True)
subprocess.run(["git", "-C", str(repo), "symbolic-ref", "HEAD", "refs/heads/main"], check=True)
subprocess.run(["git", "-C", str(repo), "repack", "-ad"], check=True)
print(repo)
