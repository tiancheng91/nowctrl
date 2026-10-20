"""Host checks for smoke-test false positives; no silicon is simulated here."""
import contextlib
import io
from pathlib import Path
import struct
import tempfile
import unittest

from p0 import check_image, check_log


class EvidenceChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "evidence"

    def test_image_requires_bootloader_partition_and_app(self):
        data = bytearray(0x11000)
        data[0] = data[0x10000] = 0xE9
        struct.pack_into("<HBBII", data, 0x8000, 0x50AA, 0, 0, 0x10000, 0x100000)
        self.path.write_bytes(data)
        with contextlib.redirect_stdout(io.StringIO()):
            check_image(self.path)
        data[0x10000] = 0
        self.path.write_bytes(data)
        with self.assertRaises(RuntimeError):
            check_image(self.path)
        self.path.write_bytes(b"\x7fELF")
        with self.assertRaises(RuntimeError):
            check_image(self.path)

    def test_log_requires_matching_role_consecutive_ticks_and_no_panic(self):
        valid = "NOWCTRL BOOT role=controller rf_channel=6\n" + "".join(
            f"NOWCTRL TICK role=controller count={n}\n" for n in [1, 2, 3]
        )
        self.path.write_text(valid)
        with contextlib.redirect_stdout(io.StringIO()):
            check_log(self.path, "controller")
        with self.assertRaises(RuntimeError):
            check_log(self.path, "node")
        for invalid in [valid.replace("count=2", "count=7"), valid + "panicked at test"]:
            self.path.write_text(invalid)
            with self.assertRaises(RuntimeError):
                check_log(self.path, "controller")


if __name__ == "__main__":
    unittest.main()
