"""Create packed fully changed text and maximum-pixel PNG review fixtures."""
import os
import pathlib
import struct
import subprocess
import sys
import zlib

os.environ.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
repo = pathlib.Path(sys.argv[1]).resolve()
repo.mkdir()
subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)

def png(color):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    width, height = 4096, 2048
    pixels = (b"\0" + bytes(color) * width) * height
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b""))

stream = bytearray()
for version in range(2):
    text = b"".join(f"{i:06d} ".encode() + bytes([97 + version]) * 56 + b"\n" for i in range(16384))
    assert len(text) == 1024 * 1024
    for mark, data in [(1 + version * 2, text), (2 + version * 2, png((40, 80 + version * 80, 120)))]:
        stream.extend(f"blob\nmark :{mark}\ndata {len(data)}\n".encode() + data + b"\n")
    message = f"Stress review {version}"
    stream.extend(f"commit refs/heads/main\ncommitter Benchmark <benchmark@example.com> {1700000000 + version} +0000\ndata {len(message)}\n{message}\n".encode())
    stream.extend(f"M 100644 :{1 + version * 2} changed.txt\nM 100644 :{2 + version * 2} image.png\n\n".encode())
subprocess.run(["git", "-C", str(repo), "fast-import", "--quiet"], input=stream, check=True)
subprocess.run(["git", "-C", str(repo), "repack", "-ad"], check=True)
print(repo)
