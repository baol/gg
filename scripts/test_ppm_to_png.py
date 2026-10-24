#!/usr/bin/env python3
import struct
import unittest
import zlib

from ppm_to_png import ppm_to_png


def chunks(data: bytes):
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise AssertionError("missing PNG signature")
    offset = 8
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        crc = struct.unpack(">I", data[offset + 8 + length : offset + 12 + length])[0]
        if crc != zlib.crc32(kind + payload) & 0xFFFFFFFF:
            raise AssertionError(f"bad CRC for {kind!r}")
        yield kind, payload
        offset += length + 12


class PpmToPngTest(unittest.TestCase):
    def test_converts_pixels_with_filter_zero_rows(self) -> None:
        pixels = bytes(
            [
                255,
                0,
                0,
                0,
                255,
                0,
                0,
                0,
                255,
                255,
                255,
                255,
            ]
        )
        png = ppm_to_png(b"P6\n2 2\n255\n" + pixels)
        parsed = dict(chunks(png))
        self.assertEqual(
            parsed[b"IHDR"], struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0)
        )
        self.assertEqual(
            zlib.decompress(parsed[b"IDAT"]),
            b"\x00" + pixels[:6] + b"\x00" + pixels[6:],
        )
        self.assertEqual(parsed[b"IEND"], b"")

    def test_accepts_comments_and_crlf_header(self) -> None:
        png = ppm_to_png(b"P6\r\n# generated\r\n1 1\r\n255\r\n\x01\x02\x03")
        parsed = dict(chunks(png))
        self.assertEqual(
            zlib.decompress(parsed[b"IDAT"]), b"\x00\x01\x02\x03"
        )

    def test_rejects_non_binary_ppm(self) -> None:
        with self.assertRaisesRegex(ValueError, "binary PPM"):
            ppm_to_png(b"P3\n1 1\n255\n0 0 0\n")

    def test_rejects_truncated_pixels(self) -> None:
        with self.assertRaisesRegex(ValueError, "expected 3"):
            ppm_to_png(b"P6\n1 1\n255\n\x01\x02")

    def test_rejects_wide_channel_values(self) -> None:
        with self.assertRaisesRegex(ValueError, "8-bit"):
            ppm_to_png(b"P6\n1 1\n65535\n\x00\x00\x00")


if __name__ == "__main__":
    unittest.main()
