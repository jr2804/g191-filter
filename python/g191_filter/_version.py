"""Build-time version stamp.

Patched by the release workflow (and scripts/install_from_git.py) together
with Cargo.toml / pyproject.toml. Kept as a literal so that `g191_filter
.__version__` works even in editable installs (`maturin develop`), where
importlib.metadata has no dist-info to read.
"""

__version__ = "0.0.0"
