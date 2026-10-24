#!/usr/bin/env python3
"""Convert a binary PPM (P6) image to PNG using only the standard library."""
from __future__ import annotations

import struct
import sys
import zlib


def _header(data: bytes) -> tuple[int, int, int]:
    if not data.startswith(b"P6"):
        raise ValueError("not a binary PPM (P6) image")
    index = 2
    tokens: list[bytes] = []
    while len(tokens) < 3:
        while index < len(data) and data[index] in b" \t\r\n":
            index += 1
        if index >= len(data):
            raise ValueError("truncated PPM header")
        if data[index] == ord("#"):
            while index < len(data) and data[index] not in b"\r\n":
                index += 1
            continue
        start = index
        while index < len(data) and data[index] not in b" \t\r\n":
            index += 1
        if start == index:
            raise ValueError("malformed PPM header")
        tokens.append(data[start:index])
    if index >= len(data) or data[index] not in b" \t\r\n":
        raise ValueError("missing PPM header terminator")
    if data[index : index + 2] == b"\r\n":
        index += 2
    else:
        index += 1
    try:
        width, height, maxval = (int(token) for token in tokens)
    except ValueError as error:
        raise ValueError("non-numeric PPM header") from error
    if width <= 0 or height <= 0:
        raise ValueError("PPM dimensions must be positive")
    if maxval != 255:
        raise ValueError("only 8-bit PPM images are supported")
    return width, height, index


def _chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def ppm_to_png(data: bytes) -> bytes:
    """Return a PNG encoding of the given P6 image bytes."""
    width, height, offset = _header(data)
    expected = width * height * 3
    pixels = data[offset:]
    if len(pixels) != expected:
        raise ValueError(f"PPM pixel data is {len(pixels)} bytes; expected {expected}")
    stride = width * 3
    raw = b"".join(
        b"\x00" + pixels[row * stride : (row + 1) * stride] for row in range(height)
    )
    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", header)
        + _chunk(b"IDAT", zlib.compress(raw, 9))
        + _chunk(b"IEND", b"")
    )


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: ppm_to_png.py INPUT.ppm OUTPUT.png", file=sys.stderr)
        return 2
    try:
        png = ppm_to_png(open(argv[1], "rb").read())
    except (OSError, ValueError) as error:
        print(f"ppm_to_png: {error}", file=sys.stderr)
        return 1
    try:
        open(argv[2], "wb").write(png)
    except OSError as error:
        print(f"ppm_to_png: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
