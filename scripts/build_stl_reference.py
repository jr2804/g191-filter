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
import shutil
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
    subprocess.run(  # noqa: S603
        ["git", "clone", "-b", BRANCH, "--depth", "1", GIT_URL, str(STL_DIR)],  # noqa: S603,S607
        check=True,
    )


def _cmake_configure(build_dir: Path) -> None:
    print(f"Configuring CMake in {build_dir} ...")
    # Prefer the platform C compiler; fall back to zig cc (needed on hosts
    # without a native toolchain, e.g. Windows without MSVC).
    cc = shutil.which("cc") or shutil.which("gcc") or shutil.which("clang")
    compiler_args = [f"-DCMAKE_C_COMPILER={cc}"] if cc else ["-DCMAKE_C_COMPILER=zig;cc"]
    subprocess.run(  # noqa: S603
        [  # noqa: S607
            "cmake",
            "..",
            *compiler_args,
            "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
        ],
        cwd=str(build_dir),
        check=True,
    )


def _cmake_build(build_dir: Path) -> None:
    print("Building STL binaries ...")
    subprocess.run(
        ["cmake", "--build", ".", "-j2"],  # noqa: S603,S607
        cwd=str(build_dir),
        check=True,
    )


def _copy_binaries(build_dir: Path) -> None:
    """Copy built STL binaries to the project root.

    CMake uses bin/Debug on MSVC/Xcode multi-config generators and bin/
    on single-config generators (Unix Makefiles, Ninja). Probe both.
    """
    dest = BASE_DIR
    suffix = ".exe" if sys.platform == "win32" else ""
    candidates = [build_dir / "bin" / "Debug", build_dir / "bin"]
    for base in ("filter", "firdemo"):
        exe = base + suffix
        src = next((d / exe for d in candidates if (d / exe).exists()), None)
        if src is None:
            print(f"  WARNING: {exe} not found in {candidates}")
            continue
        dest_path = dest / exe
        if dest_path.exists() and dest_path.stat().st_size >= src.stat().st_size:
            print(f"  {exe} already up to date")
        else:
            shutil.copy2(src, dest_path)
            print(f"  copied {exe} from {src}")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as exc:
        print(f"Build failed: {exc}", file=sys.stderr)
        sys.exit(1)
