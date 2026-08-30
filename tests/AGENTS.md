# Tests boundary

Pytest test suite for the `g191_filter` package.

## Purpose

Unit and integration tests that validate the Rust DSP core and its alignment
with the ITU-T G.191 STL reference implementation.

## Structure

| File | Scope |
| ---- | ------ |
| `test_basic_filtering.py` | Rust core unit tests (import, shape, idempotence, real inputs, coefficient availability) |
| `test_reference_verification.py` | Integration tests vs STL reference + openitu sanity/precision |
| `test_template_validation.py` | Project scaffold validation (pyproject.toml, pytest, mise) |
| `conftest.py` | Shared fixtures (fixtures are defined in each test file) |

## Running

```bash
python -m pytest tests/
```

## Reference-code integration tests

`test_reference_verification.py` clones the STL reference from GitHub on demand
(branch `STL2026_ITU-T_submission`), builds `filter.exe`, and compares Rust
`filter_array()` output against the C reference.

- `scripts/build_stl_reference.py` — clones + builds the STL reference
- `scripts/clone_stl.sh` — clones only (no build)

The `filter.exe` comparison is limited to filters whose C reference produces
the same output length as the Rust core: **IRS8, IRS16, HQ2, HQ3**. Other
filters (PCM, FLAT, DC) decimate internally in the C reference but keep the
native sample rate in the Rust API, so they are skipped.

A ±1 sample tolerance accounts for C `float` (32-bit) vs Rust `f64` coefficient precision.

## openitu STL tests

`test_openitu_sanity` and `test_openitu_precision` run the `basop_test.exe`
runner from the built STL reference (CMake target `basop_test`).
