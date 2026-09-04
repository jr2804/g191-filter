"""g191_filter package — Rust-backed ITU-T G.191 FIR/IIR filter library.

The compiled Rust extension ships as `g191_filter.g191_filter.{so,pyd,dylib}`
under this package; the symbols are re-exported here as top-level.
"""

from __future__ import annotations

import importlib.metadata

from .g191_filter import (  # noqa: F401
    BlockwiseFilter,
    export_impulse_response,
    filter_array,
    filter_wave,
    frequency_response_scan,
    get_coefficients_ba_py,
    get_coefficients_sos_py,
    get_filter_info_py,
    get_frequency_response,
    list_filters,
)

get_filter_info = get_filter_info_py  # alias

try:
    __version__ = importlib.metadata.version("g191-filter")
except importlib.metadata.PackageNotFoundError:
    # Editable install (maturin develop without --uv): no dist-info.
    from ._version import __version__ as __version__
