"""Typer CLI app for ITU-T G.191 FIR/IIR signal filters."""

from __future__ import annotations

from importlib.metadata import version

import typer

from g191_filter.cli import commands

app = typer.Typer(
    name="g191_filter",
    help="IIR/FIR filters according to Recommendation ITU-T G.191 (Software Tool Library). Provided at arbitrary sampling rates and in many formats",
    add_completion=True,
    no_args_is_help=True,
)

# Commands are plain functions in g191_filter.cli.commands; wiring them
# here (after app exists) keeps the module free of circular imports, so
# csort may freely reorder the imports section without breaking them.
app.command()(commands.default)
app.command()(commands.filter)

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


def main() -> None:
    """Entry point for the CLI application."""
    app()
