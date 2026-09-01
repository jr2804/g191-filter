"""CLI arguments, options, and flags for ITU-T G.191 FIR/IIR signal filters."""

from __future__ import annotations

from typing import Annotated

import typer

# Application name for environment variables
APP_NAME_UPPERCASE = "G191_FILTER"

# Filter command arguments
FilterIdArg = Annotated[
    str,
    typer.Argument(help="ITU-T G.191 filter ID (case-insensitive)"),
]

InputWaveFile = Annotated[
    str,
    typer.Argument(help="Path to input WAV file"),
]

OutputFileArg = Annotated[
    str,
    typer.Option(
        "--output-file",
        "-o",
        help="Output WAV file path (default: overwrite input)",
        envvar=f"{APP_NAME_UPPERCASE}_OUTPUT_FILE",
    ),
]

BlockSizeOption = Annotated[
    int,
    typer.Option(
        "--block-size",
        "-b",
        help="Chunk size for streaming processing (default: 8192)",
        min=1,
    ),
]
