#!/usr/bin/env python3
"""Clone the ITU-T G.191 STL reference code and build the filter/firdemo binaries.

The STL reference is fetched from the openitu GitHub mirror on the
STL2026_ITU-T_submission branch and built with CMake.

Artifacts produced:
    filter.exe  — standalone single-filter test program
    firdemo.exe — cascading multi-filter demo program
"""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parents[1]
STL_DIR = BASE_DIR / "tmp" / "_stl_extract"
GIT_URL = "https://github.com/openitu/STL.git"
BRANCH = "STL2026_ITU-T_submission"


def main() -> None:
    if STL_DIR.exists():
        print(f"STL extract already present at {STL_DIR}")
    else:
        _clone()

    build_dir = STL_DIR / "build"
    build_dir.mkdir(parents=True, exist_ok=True)
    _cmake_configure(build_dir)
    _cmake_build(build_dir)
    _copy_binaries(build_dir)
    print("STL reference build complete.")


def _clone() -> None:
    print(f"Cloning STL reference from {GIT_URL} branch {BRANCH} ...")
    os.makedirs(str(STL_DIR.parent), exist_ok=True)
    subprocess.run(
        ["git", "clone", "-b", BRANCH, "--depth", "1", GIT_URL, str(STL_DIR)],
        check=True,
    )


def _cmake_configure(build_dir: Path) -> None:
    print(f"Configuring CMake in {build_dir} ...")
    subprocess.run(
        [
            "cmake",
            "..",
            "-DCMAKE_C_COMPILER=zig;cc",
            "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
        ],
        cwd=str(build_dir),
        check=True,
    )


def _cmake_build(build_dir: Path) -> None:
    print(f"Building STL binaries ...")
    subprocess.run(
        ["cmake", "--build", ".", "-j2"],
        cwd=str(build_dir),
        check=True,
    )


def _copy_binaries(build_dir: Path) -> None:
    """Copy built STL binaries to the project root."""
    bin_dir = build_dir / "bin" / "Debug"
    dest = BASE_DIR
    suffix = ".exe" if sys.platform == "win32" else ""
    for base in ("filter", "firdemo"):
        exe = base + suffix
        src = bin_dir / exe
        if src.exists():
            dest_path = dest / exe
            if dest_path.exists() and dest_path.stat().st_size >= src.stat().st_size:
                print(f"  {exe} already up to date")
            else:
                import shutil
                shutil.copy2(src, dest_path)
                print(f"  copied {exe}")
        else:
            print(f"  WARNING: {exe} not found in {bin_dir}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as exc:
        print(f"Build failed: {exc}", file=sys.stderr)
        sys.exit(1)
