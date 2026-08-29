"""g191_filter package."""

from __future__ import annotations

import importlib.metadata

from _native import (  # noqa: F401
    BlockwiseFilter,
    export_impulse_response,
    filter_array,
    filter_wave,
    get_coefficients_ba_py,
    get_coefficients_sos_py,
    get_frequency_response,
    list_filters,
)

try:
    __version__ = importlib.metadata.version(__name__)
except importlib.metadata.PackageNotFoundError:
    __version__ = "0.0.0"  # Fallback for development mode
