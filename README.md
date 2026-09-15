# ITU-T G.191 FIR & IIR Signal Filters

> Standard speech and audio processing filters from ITU-T Recommendation G.191
> (Software Tool Library), implemented in a high-performance Rust core with Python bindings and CLI.

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="#installation"><img alt="Python 3.13, 3.14, 3.14t" src="https://img.shields.io/badge/python-3.13%20%7C%203.14%20%7C%203.14t-3776ab?logo=python&logoColor=white"></a>
  <a href="https://github.com/jr2804/g191-filter/releases"><img alt="Free-threaded 3.14t" src="https://img.shields.io/badge/no--GIL-3.14t-30638b?logo=python&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-green.svg"></a>
  <a href="https://github.com/jr2804/g191-filter/actions"><img alt="CI" src="https://github.com/jr2804/g191-filter/actions/workflows/ci.yml/badge.svg"></a>
</p>
<!-- markdownlint-enable MD033 -->

---

## Overview

ITU-T Recommendation G.191 specifies the standard reference DSP filters used across speech codecs,
telephony equipment evaluation, perceptual audio quality testing (e.g. PESQ, POLQA), and telecom standardization:

- **IRS (Intermediate Reference System)**: Standard 8 kHz, 16 kHz, and Modified IRS 16/48 kHz sending/receiving curves simulating handset frequency characteristics.
- **Low-Pass Suite**: 48 kHz linear-phase FIR band-limiting filters, named after the ITU-T band
  edges they are designed to constrain to (2, 4, 8, 12, 14, 16 and 24 kHz). They roll off
  gradually — measured attenuation at the intended edge is −8.8 to −19.1 dB
  ([specifications](https://jr2804.github.io/g191-filter/reference/filters/#specifications)).
- **Resampling & Rate-Conversion**: High-quality 2:1/3:1 FIR/HQ decimation, 1:2/1:3 FIR up-sampling, and IIR 3:1 direct/cascade stages with integrated rate change.
- **Standard PCM**: Standard PCM weighting filters (parallel-form IIR, 16 kHz design) with 1:1, 2:1 downsampling, and 1:2 upsampling variants.
- **Weighting & Measurement**: ITU-T measurement weightings including psophometric noise, P.341, half-tilt IRS, TIA-IRS, and delta-sigma modulation.
- **Wideband Band-Pass**: Linear-phase filters from narrowband (50 Hz) to fullband (20 kHz) at 16/32/48 kHz.

This package provides:

- **Rust core**: Zero-allocation inner loops, polyphase downsampling/upsampling kernels, and exact coefficient structures.
- **Python API (`numpy`)**: One-shot batch processing, stateful streaming (`BlockwiseFilter`), impulse response generation, and coefficient access (`b, a` and `SOS`).
- **Command-line Interface**: Streaming WAV file filtering with zero extra setup.

---

## Installation

Requires **Python 3.13 or newer**. Prebuilt wheels need no Rust toolchain and cover
CPython 3.13, 3.14 and 3.14t on Linux (x86_64, aarch64), macOS (arm64, x86_64) and
Windows (x64) — 15 wheels per release, all attached to the
[release page](https://github.com/jr2804/g191-filter/releases).

```bash
# Prebuilt wheel for your platform (adjust the tag and the platform tag)
uv pip install "https://github.com/jr2804/g191-filter/releases/download/2026.9.13/g191_filter-2026.9.13-cp313-cp313-win_amd64.whl"

# Add to a project, pinned to a release tag…
uv add "git+https://github.com/jr2804/g191-filter.git@2026.9.13"

# …or follow the tip of main
uv add "git+https://github.com/jr2804/g191-filter.git"
```

Installing from git compiles the Rust core through the `maturin` build backend, so those
two forms need a Rust toolchain. The package is not published to PyPI — installs come
from the release wheels or from git.

### Version reported by a git install

Every push to `main` is released: the release workflow stamps the CalVer into
`Cargo.toml`, `Cargo.lock`, `pyproject.toml` and `python/g191_filter/_version.py`,
commits that to `main`, and tags that commit. A git install therefore reports the
version of the release it was built from — head of `main` reports the most recent
release, and a tag-pinned install reports exactly that tag. There is no `0.0.0`
placeholder to work around for either case.

---

## Quick Usage

Run without installing via `uvx` or `uv run`:

### Command-Line Interface (CLI)

```bash
# Filter a WAV file using the IRS 8 kHz filter directly from GitHub
uvx --from git+https://github.com/jr2804/g191-filter.git g191-filter filter \
  irs8khz speech.wav --output-file speech_irs.wav

# Pin to a release tag so the wheel version matches the source you cloned:
uvx --from "git+https://github.com/jr2804/g191-filter.git@2026.9.13" g191-filter filter \
  irs8khz speech.wav --output-file speech_irs.wav

# Or inside a project environment with uv run:
uv run --with git+https://github.com/jr2804/g191-filter.git g191-filter filter \
  mod_irs16khz wideband.wav --output-file filtered.wav --block-size 4096
```

`filter` takes the filter ID and the input WAV as positional arguments; run
`g191-filter filter --help` for the full option list (`--output-file`, `--block-size`).

### Python API

```python
import numpy as np
from g191_filter import BlockwiseFilter, filter_array, filter_wave, list_filters

# 1. Inspect available filters
print(list_filters())
# ['hq_down_2_to_1', 'hq_down_3_to_1', 'hq_up_1_to_2', 'hq_up_1_to_3', 'flat_band_pass', 'flat1', ...]

# 2. One-shot filtering on a NumPy array
signal = np.random.randn(16000)
filtered = filter_array("irs16khz", signal)

# 3. Stateful blockwise / streaming filtering
bw = BlockwiseFilter("mod_irs16khz", block_size=1024)
chunk_out = bw.process(signal[:1024])

# 4. Filter a WAV file directly
filter_wave("lp7_48khz", "input_48k.wav", output_file="output_lp7.wav")
```

---

## Filter Families

| Family | Filter IDs | Description | Typical Rate |
| ------ | ---------- | ----------- | ------------ |
| **IRS Family** | `irs8khz`, `irs16khz`, `mod_irs16khz`, `mod_irs48khz` | Intermediate Reference System telephony handset responses | 8 / 16 / 48 kHz |
| **48 kHz Low-Pass** | `lp1p5_48khz`, `lp35_48khz`, `lp7_48khz`, `lp10_48khz`, `lp12_48khz`, `lp14_48khz`, `lp20_48khz` | 48 kHz linear-phase band-limiting shaping filters (gradual roll-off) | 48 kHz |
| **Resampling** | `hq_down_2_to_1`, `hq_down_3_to_1`, `hq_up_1_to_2`, `hq_up_1_to_3`, `flat_1_to_2`, `flat1`, `iir_down_3_to_1`, `iir_up_1_to_3`, `iir_casc_lp_3_to_1`, `iir_casc_lp_1_to_3` | Decimation & interpolation filters with integrated rate change | 8 / 16 / 48 kHz |
| **Telecom & DC** | `flat_band_pass`, `g712_8khz`, `stdpcm_16khz`, `stdpcm_2_to_1`, `stdpcm_1_to_2`, `dir_dc_removal` | Flat 300–3400 Hz bandpass, G.712 PCM channel filter (parallel-form IIR, with 2:1/1:2 rate variants), and DC block | 8 / 16 kHz |
| **Weighting & Measurement** | `msin16khz`, `psophometric_8khz`, `dsm16khz`, `hirs16khz`, `tia_irs8khz`, `rx_irs8khz`, `rx_irs16khz`, `p341_16khz` | ITU-T measurement weightings: psophometric noise, P.341, half-tilt IRS, TIA-IRS, delta-SM | 8 / 16 kHz |
| **Band-Pass (Wideband)** | `bp5k_16khz`, `bp100_5k_16khz`, `bp14k_32khz`, `bp20k_48khz` | Linear-phase band-pass filters from narrowband up to fullband (20 Hz–20 kHz) | 16 / 32 / 48 kHz |

Detailed frequency response curves, coefficient specifications, and parameter references are available in the [Documentation](https://jr2804.github.io/g191-filter/).

---

## Scope & Limitations

- Only the IIR/FIR filters specified in ITU-T G.191 are implemented, addressed by the filter IDs given in the recommendation — no additional filters are added.
- The 48 kHz low-pass suite is a family of band-limiting *shaping* filters with a gradual roll-off,
  not brick-wall anti-aliasing. The steep filters are the re-sampling family (`hq_down_*`,
  `iir_casc_lp_*`); per-filter measured attenuation is in the
  [specifications](https://jr2804.github.io/g191-filter/reference/filters/#specifications).
- `g191-filter filter` overwrites the input WAV unless `--output-file` is given.

---

## Documentation & Development

Full guides and references:

- [Filter Catalog & Specifications](https://jr2804.github.io/g191-filter/reference/filters/)
- [One-Shot & Batch Filtering Guide](https://jr2804.github.io/g191-filter/guides/one_shot_filtering/)
- [Blockwise & Streaming Filtering Guide](https://jr2804.github.io/g191-filter/guides/blockwise_filtering/)
- [API Reference](https://jr2804.github.io/g191-filter/reference/api/)
- [Development & Contributing](https://jr2804.github.io/g191-filter/development/)

Bugs, questions and filter requests: [open an issue](https://github.com/jr2804/g191-filter/issues).

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
Coefficients and filter algorithms conform to Recommendation ITU-T G.191.
