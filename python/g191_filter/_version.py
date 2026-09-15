"""Build-time version stamp.

Stamped by the release workflow together with Cargo.toml / Cargo.lock /
pyproject.toml, and committed to main before the tag is created. Kept as a
literal so that `g191_filter.__version__` reports the right value even in
editable installs (`maturin develop`), where importlib.metadata has no
dist-info to read.
"""

__version__ = "2026.9.10"
