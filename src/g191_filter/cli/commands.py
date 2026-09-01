"""CLI command implementations for ITU-T G.191 FIR/IIR signal filters."""

from __future__ import annotations

import typer

from g191_filter import filter_wave
from g191_filter.cli import args

# Default command (runs when no command provided)


def default() -> None:
    """Default command showing welcome message."""
    typer.echo("Welcome to ITU-T G.191 FIR/IIR signal filters!")
    typer.echo("Use --help to see available commands.")


# Filter command


def filter(
    filter_id: args.FilterIdArg,
    input_file: args.InputWaveFile,
    output_file: args.OutputFileArg = None,
    block_size: args.BlockSizeOption = 8192,
) -> None:
    """Apply a G.191 filter to a WAV file in chunks (streaming)."""
    out_path = output_file if output_file else input_file
    filter_wave(filter_id, input_file, output_file=out_path, block_size=block_size)
    typer.echo(f"Filtered {input_file} -> {out_path} with {filter_id} (block_size={block_size})")
