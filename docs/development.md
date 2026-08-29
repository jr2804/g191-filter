---
title: Development & Contributing
---

## Development Guide

This guide covers setting up the development environment, building the native Rust core, running tests, linting, and building documentation.

### Environment Setup

The project uses [`mise`](https://mise.jdx.dev/) for task orchestration and [`uv`](https://docs.astral.sh/uv/) for Python package management.
The numerical filtering core is written in Rust and compiled via Maturin / PyO3.

```bash
# Clone the repository
git clone https://github.com/jr2804/g191-filter.git
cd g191-filter

# Set up tools and sync virtual environment
mise dev
```

### Native Extension Build Workflow

The Python interface binds to the Rust crate `_native` compiled via PyO3:

```bash
# Compile release binary and place into site-packages
uvx maturin develop --release
```

### Running Tests

Integration and unit tests verify filter coefficients, frequency responses, state consistency, and output equivalence against the ITU-T reference suite.

```bash
# Run tests with pytest
uv run pytest

# Or via mise task
mise test
```

### Code Quality & Formatting

```bash
mise lint       # Runs ruff, ty type-checker, and codespell
mise format     # Runs ruff format and isort
mise all        # Combined test, lint, and format verification
```

### CLI Development

Run the Typer CLI in development mode:

```bash
# Show command overview
uv run g191-filter --help

# Stream filter a WAV file
uv run g191-filter filter --filter-id irs8khz --input-file input.wav --output-file output.wav
```

### Documentation Workflow

Documentation is built with Zensical (MkDocs-compatible theme engine):

```bash
# Local preview with live-reload
mise docs-serve    # preview at http://localhost:8000

# Strict static build
mise docs-build    # outputs to site/
```

#### Regenerating Filter Figures

Vector response figures embedded in the documentation are dynamically plotted directly from the filter core using the `xy` package:

```bash
uv run python scripts/gen_filter_figures.py
```

### CI/CD Pipeline

| Workflow | Trigger | Scope |
| -------- | ------- | ----- |
| **CI** (`ci.yml`) | Push / PR to `main` | Python matrix (3.13, 3.14) + Rust build + lint + test suite |
| **Docs** (`docs.yml`) | Push to `main` | Renders figures, builds static site, deploys to GitHub Pages |
| **Release** (`release.yml`) | Tag `v*` | Wheel compilation and deployment |
