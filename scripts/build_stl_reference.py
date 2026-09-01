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


def main(stl_dir: Path | None = None, out_dir: Path | None = None) -> None:
    stl_dir = stl_dir or STL_DIR
    out_dir = out_dir or BASE_DIR
    if stl_dir.exists():
        print(f"STL extract already present at {stl_dir}")
    else:
        _clone(stl_dir)

    build_dir = stl_dir / "build"
    build_dir.mkdir(parents=True, exist_ok=True)
    _cmake_configure(build_dir)
    _cmake_build(build_dir)
    _copy_binaries(build_dir, out_dir)
    print("STL reference build complete.")


def _clone(stl_dir: Path) -> None:
    print(f"Cloning STL reference from {GIT_URL} branch {BRANCH} ...")
    os.makedirs(str(stl_dir.parent), exist_ok=True)
    subprocess.run(  # noqa: S603
        ["git", "clone", "-b", BRANCH, "--depth", "1", GIT_URL, str(stl_dir)],  # noqa: S603,S607
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


def _copy_binaries(build_dir: Path, out_dir: Path) -> None:
    """Copy built STL binaries to out_dir (default: project root).

    CMake uses bin/Debug on MSVC/Xcode multi-config generators and bin/
    on single-config generators (Unix Makefiles, Ninja). Probe both.
    """
    dest = out_dir
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
    import argparse

    parser = argparse.ArgumentParser(description="Clone + build the ITU-T STL reference binaries")
    parser.add_argument("--stl-dir", type=Path, default=STL_DIR,
                        help="Directory for the STL clone+build (default: tmp/_stl_extract)")
    parser.add_argument("--out-dir", type=Path, default=BASE_DIR,
                        help="Directory to copy built binaries to (default: project root)")
    args = parser.parse_args()
    try:
        main(stl_dir=args.stl_dir, out_dir=args.out_dir)
    except subprocess.CalledProcessError as exc:
        print(f"Build failed: {exc}", file=sys.stderr)
        sys.exit(1)
