"""Typer CLI app for ITU-T G.191 FIR/IIR signal filters."""

from __future__ import annotations

from importlib.metadata import version

import typer

app = typer.Typer(
    name="g191_filter",
    help="IIR/FIR filters according to Recommendation ITU-T G.191 (Software Tool Library). Provided at arbitrary sampling rates and in many formats",
    add_completion=True,
    no_args_is_help=True,
)


@app.callback(invoke_without_command=True)
def _callback(
    version: bool = typer.Option(
        False,
        "--version",
        "-v",
        help="Show version and exit",
        is_eager=True,
    ),
) -> None:
    """IIR/FIR filters according to Recommendation ITU-T G.191 (Software Tool Library). Provided at arbitrary sampling rates and in many formats"""
    if version:
        typer.echo(_get_version())
        raise typer.Exit()


# Version management
def _get_version() -> str:
    """Get application version from package metadata."""
    try:
        return version("g191_filter")
    except Exception:
        return "0.0.0"  # Fallback for development mode


# Import commands to register them with app (after app exists)
from g191_filter.cli import commands  # noqa: E402, F401


def main() -> None:
    """Entry point for the CLI application."""
    app()
