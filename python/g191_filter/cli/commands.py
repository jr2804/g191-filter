"""CLI command implementations for ITU-T G.191 FIR/IIR signal filters."""

from __future__ import annotations

from typing import Annotated

import typer

from g191_filter import filter_wave, frequency_response_scan
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


# Frequency response scan (STL fltresp method)
def freqresp(
    filter_id: args.FilterIdArg,
    f0: Annotated[float, typer.Option("--f0", help="Starting frequency in Hz")] = 50.0,
    ff: Annotated[float, typer.Option("--ff", help="Final frequency in Hz")] = None,
    fstep: Annotated[float, typer.Option("--fstep", help="Step in Hz")] = 50.0,
    sample_rate: Annotated[int, typer.Option("--fs", help="Sample rate in Hz")] = 8000,
) -> None:
    """Sine-power frequency response scan (STL fltresp method)."""
    if ff is None:
        ff = sample_rate / 2.0 - 1
    freqs, gains = frequency_response_scan(
        filter_id,
        f0 / sample_rate,
        ff / sample_rate,
        fstep / sample_rate,
        float(sample_rate),
    )
    typer.echo(f"# filter={filter_id} fs={sample_rate} f0={f0} ff={ff} fstep={fstep}")
    typer.echo("# freq_hz	gain_db")
    for f, g in zip(freqs, gains, strict=False):
        typer.echo(f"{f:.2f}	{g:.3f}")
