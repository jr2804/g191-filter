"""Version information for ITU-T G.191 FIR/IIR signal filters."""
from __future__ import annotations

import importlib.metadata

from ._version import __version__ as _stamped_version

__title__ = "g191_filter"
__description__ = "IIR/FIR filters according to Recommendation ITU-T G.191 (Software Tool Library). Provided at arbitrary sampling rates and in many formats"
try:
    __version__ = importlib.metadata.version("g191-filter")
except importlib.metadata.PackageNotFoundError:
    # Editable install (maturin develop without --uv): no dist-info.
    __version__ = _stamped_version

__author__ = "Jan.Reimes"
__email__ = "jan.reimes@head-acoustics.com"
__license__ = "MIT"
__copyright__ = "Copyright 2026, Jan.Reimes"
