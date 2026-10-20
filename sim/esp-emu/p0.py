#!/usr/bin/env python3
"""Reproducible P0 build, merged-image generation and bounded emulator smoke test."""
import argparse
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
VERSIONS = {"espflash": "4.4.0", "esp-emu": "0.49.0"}


def tool(name):
    executable = os.environ.get(name.upper().replace("-", "_") + "_BIN", name)
    if not shutil.which(executable):
        raise RuntimeError(f"Missing {name}: put it on PATH or set {name.upper().replace('-', '_')}_BIN")
    version = subprocess.check_output([executable, "--version"], text=True)
    if not re.search(r"\b" + re.escape(VERSIONS[name]) + r"\b", version):
        raise RuntimeError(f"Expected {name} {VERSIONS[name]}, got {version.strip()}")
    return executable


def check_image(path):
    data = path.read_bytes()
    # ESP32-C3 bootloader at 0, default partition table at 0x8000.
    if len(data) <= 0x10000 or data[0] != 0xE9:
        raise RuntimeError("Missing bootloader image magic at 0")
    apps = []
    for offset in range(0x8000, 0x8C00, 32):
        magic, kind, subtype, start, size = struct.unpack_from("<HBBII", data, offset)
        if magic != 0x50AA:
            break
        if kind == 0:
            apps.append((start, size))
    if len(apps) != 1 or apps[0][0] != 0x10000:
        raise RuntimeError(f"Unexpected application partitions: {apps}")
    if data[0x10000] != 0xE9 or len(data) > 4 * 1024 * 1024:
        raise RuntimeError("Invalid application magic or oversized 4MB flash image")
    print(f"IMAGE verified bootloader=0x0 partitions=0x8000 app=0x10000 bytes={len(data)}", flush=True)


def check_log(path, role):
    text = path.read_text(errors="replace")
    if f"NOWCTRL BOOT role={role} rf_channel=6" not in text:
        raise RuntimeError("Startup marker missing; inspect emulator log")
    ticks = [int(x) for x in re.findall(rf"NOWCTRL TICK role={role} count=(\d+)", text)]
    if not any(ticks[i:i + 3] == list(range(ticks[i], ticks[i] + 3)) for i in range(len(ticks) - 2)):
        raise RuntimeError(f"Three consecutive timer ticks missing: {ticks}")
    if re.search(r"panicked at|Exception:|Stack overflow", text, re.I):
        raise RuntimeError("Firmware fault found; inspect emulator log")
    print(f"SMOKE PASS role={role} ticks={len(ticks)} (radio delivery is not certified)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["build", "run", "check"])
    parser.add_argument("role", choices=["controller", "node"])
    parser.add_argument("--seconds", type=int, default=20)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60:
        parser.error("--seconds must be between 1 and 60")
    output = ROOT / "target" / "p0" / args.role
    output.mkdir(parents=True, exist_ok=True)
    image = output / "merged.bin"
    log = output / "emulator.log"
    if args.action == "build":
        espflash = tool("espflash")
        command = ["cargo", "build", "--locked", "--release"]
        if args.role == "node":
            command += ["--no-default-features", "--features", "node"]
        print("BUILD " + " ".join(command), flush=True)
        subprocess.run(command, cwd=ROOT, check=True)
        elf = output / "firmware.elf"
        shutil.copy2(ROOT / "target/riscv32imc-unknown-none-elf/release/nowctrl", elf)
        subprocess.run([espflash, "save-image", "--chip", "esp32c3", "--flash-size", "4mb",
                        "--merge", "--skip-update-check", str(elf), str(image)], cwd=ROOT, check=True)
        check_image(image)
    elif args.action == "run":
        emulator = tool("esp-emu")
        check_image(image)
        command = [emulator, "--chip", "esp32c3", "--firmware", str(image),
                   "--timeout", f"{args.seconds}s", "--console", "usb-jtag", "--log-color", "never"]
        with log.open("w") as stream:
            stream.write("COMMAND " + " ".join(command) + "\n")
            stream.flush()
            try:
                result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT,
                                        timeout=args.seconds + 10)
            except subprocess.TimeoutExpired as error:
                raise RuntimeError(f"Host timeout; partial log preserved at {log}") from error
        print(f"Emulator exit={result.returncode}; log={log}")
        if result.returncode:
            raise RuntimeError("Emulator exited unsuccessfully; inspect log")
        check_log(log, args.role)
    else:
        check_log(log, args.role)


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError, subprocess.CalledProcessError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
