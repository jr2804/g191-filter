"""Install g191-filter from a git checkout with the right version tag.

`uv add git+https://github.com/jr2804/g191-filter.git` (or
`pip install git+https://...`) clones the repo and runs maturin directly,
which reads `[package].version` from `Cargo.toml`. The committed value is
`0.0.0` (intentional placeholder for local development), so the resulting
wheel reports `g191-filter==0.0.0` regardless of which commit you pinned.

For tagged releases (`uv add git+...@v2026.9.11`) the release workflow
already patches `Cargo.toml` to the correct CalVer, so that path works
without this script.

Use this helper when installing from a non-tagged commit (a branch,
a specific SHA, or HEAD):

    uv run python scripts/install_from_git.py

It reads the most recent reachable CalVer git tag, updates `Cargo.toml`
in place, then invokes `maturin develop --release` so the wheel the
local venv installs matches the real commit you asked for.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO_TOML = ROOT / "Cargo.toml"
TAG_PATTERN = re.compile(r"^(\d{4}\.\d{1,2}\.\d+)$")
_GIT_BIN = shutil.which("git") or "git"


def _git_version(cwd: Path) -> str | None:
    """Return the cleanest CalVer version reachable from HEAD, or None."""
    try:
        out = subprocess.check_output(  # noqa: S603
            [_GIT_BIN, "describe", "--tags", "--abbrev=0"],
            cwd=cwd,
            stderr=subprocess.PIPE,
            shell=False,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    tag = out.decode().strip().lstrip("v")
    return tag if TAG_PATTERN.match(tag) else None


def _set_cargo_version(version: str) -> None:
    """Replace the `[package] version = "..."` line in Cargo.toml."""
    text = CARGO_TOML.read_text(encoding="utf-8")
    new_text, count = re.subn(
        r'^(version\s*=\s*)"[^"]*"',
        rf'\1"{version}"',
        text,
        count=1,
        flags=re.MULTILINE,
    )
    if count != 1:
        sys.exit(f'Could not find `version = "..."` line at top of {CARGO_TOML}; set it to "{version}" manually and rerun.')
    CARGO_TOML.write_text(new_text, encoding="utf-8")


def _build(args: list[str]) -> int:
    cmd = ["uv", "run", "maturin", "develop", "--release", *args]
    return subprocess.call(cmd, cwd=ROOT)  # noqa: S603


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--skip-build",
        action="store_true",
        help="Patch Cargo.toml only; do not invoke maturin develop.",
    )
    p.add_argument(
        "--version",
        help="Override the discovered version (otherwise read from git tag).",
    )
    p.add_argument(
        "maturin_args",
        nargs=argparse.REMAINDER,
        default=[],
        help="Extra arguments forwarded to `maturin develop --release`.",
    )
    ns = p.parse_args()

    version = ns.version or _git_version(ROOT)
    if not version:
        sys.exit(f"No CalVer tag reachable from HEAD in {ROOT}; pass --version <YYYY.M.N> or create a tag first.")

    print(f"Detected version: {version}")
    print(f"Patching {CARGO_TOML}")
    _set_cargo_version(version)

    if ns.skip_build:
        print("--skip-build set; not invoking maturin develop.")
        return

    print("Running maturin develop --release")
    sys.exit(_build(ns.maturin_args))


if __name__ == "__main__":
    main()
