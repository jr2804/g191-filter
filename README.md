# ITU-T G.191 FIR & IIR Signal Filters

> Standard speech and audio processing filters from ITU-T Recommendation G.191
> (Software Tool Library), implemented in a high-performance Rust core with Python bindings and CLI.

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="#"><img alt="Python 3.13, 3.14, 3.14t" src="https://img.shields.io/badge/python-3.13%20%7C%203.14%20%7C%203.14t-3776ab?logo=python&logoColor=white"></a>
  <a href="#"><img alt="Free-threaded 3.14t" src="https://img.shields.io/badge/no--GIL-3.14t-30638b?logo=python&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-green.svg"></a>
  <a href="https://github.com/jr2804/g191-filter/actions"><img alt="CI" src="https://github.com/jr2804/g191-filter/actions/workflows/ci.yml/badge.svg"></a>
</p>
<!-- markdownlint-enable MD033 -->

---

## Overview

ITU-T Recommendation G.191 specifies the standard reference DSP filters used across speech codecs,
telephony equipment evaluation, perceptual audio quality testing (e.g. PESQ, POLQA), and telecom standardization:

- **IRS (Intermediate Reference System)**: Standard 8 kHz, 16 kHz, and Modified IRS 16/48 kHz sending/receiving curves simulating handset frequency characteristics.
- **Low-Pass Suite**: Precise 48 kHz linear-phase FIR filters with cutoffs at 1.5, 3.5, 7.0, 10.0, 12.0, 14.0, and 20.0 kHz.
- **Resampling & Rate-Conversion**: High-quality 2:1/3:1 FIR/HQ decimation, 1:2/1:3 FIR up-sampling, and IIR 3:1 direct/cascade stages with integrated rate change.
- **Standard PCM**: Standard PCM weighting filters (parallel-form IIR, 16 kHz design) with 1:1, 2:1 downsampling, and 1:2 upsampling variants.
- **Weighting & Measurement**: ITU-T measurement weightings including psophometric noise, P.341, half-tilt IRS, TIA-IRS, and delta-sigma modulation.
- **Wideband Band-Pass**: Linear-phase filters from narrowband (50 Hz) to fullband (20 kHz) at 16/32/48 kHz.

This package provides:

- **Rust core**: Zero-allocation inner loops, polyphase downsampling/upsampling kernels, and exact coefficient structures.
- **Python API (`numpy`)**: One-shot batch processing, stateful streaming (`BlockwiseFilter`), impulse response generation, and coefficient access (`b, a` and `SOS`).
- **Command-line Interface**: Streaming WAV file filtering with zero extra setup.

---

## Quick Usage

Run without installing via `uvx` or `uv run`:

### Command-Line Interface (CLI)

```bash
# Filter a WAV file using the IRS 8 kHz filter directly from GitHub
uvx --from git+https://github.com/jr2804/g191-filter.git g191-filter filter \
  --filter-id irs8khz \
  --input-file speech.wav \
  --output-file speech_irs.wav

# Pin to a release tag so the wheel version matches the source you cloned:
uvx --from "git+https://github.com/jr2804/g191-filter.git@v2026.9.13" g191-filter filter \
  --filter-id irs8khz --input-file speech.wav --output-file speech_irs.wav

# Or inside a project environment with uv run:
uv run --from git+https://github.com/jr2804/g191-filter.git g191-filter filter \
  --filter-id mod_irs16khz \
  --input-file wideband.wav \
  --output-file filtered.wav \
  --block-size 4096
```

### Python API

```python
import numpy as np
from g191_filter import BlockwiseFilter, filter_array, filter_wave, list_filters

# 1. Inspect available filters
print(list_filters())
# ['hq_down_2_to_1', 'hq_down_3_to_1', 'flat_band_pass', 'irs8khz', 'irs16khz', ...]

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
| **48 kHz Low-Pass** | `lp1p5_48khz`, `lp35_48khz`, `lp7_48khz`, `lp10_48khz`, `lp12_48khz`, `lp14_48khz`, `lp20_48khz` | Linear-phase anti-aliasing / band-limiting low-pass filters | 48 kHz |
| **Resampling** | `hq_down_2_to_1`, `hq_down_3_to_1`, `hq_up_1_to_2`, `hq_up_1_to_3`, `flat_1_to_2`, `flat1`, `iir_down_3_to_1`, `iir_up_1_to_3`, `iir_casc_lp_3_to_1`, `iir_casc_lp_1_to_3` | Decimation & interpolation filters with integrated rate change | 8 / 16 / 48 kHz |
| **Telecom & DC** | `flat_band_pass`, `g712_8khz`, `stdpcm_16khz`, `stdpcm_2_to_1`, `stdpcm_1_to_2`, `dir_dc_removal` | Flat 300–3400 Hz bandpass, G.712 PCM channel filter (parallel-form IIR, with 2:1/1:2 rate variants), and DC block | 8 / 16 kHz |
| **Weighting & Measurement** | `msin16khz`, `psophometric_8khz`, `dsm16khz`, `hirs16khz`, `tia_irs8khz`, `rx_irs8khz`, `rx_irs16khz`, `p341_16khz` | ITU-T measurement weightings: psophometric noise, P.341, half-tilt IRS, TIA-IRS, delta-SM | 8 / 16 kHz |
| **Band-Pass (Wideband)** | `bp5k_16khz`, `bp100_5k_16khz`, `bp14k_32khz`, `bp20k_48khz` | Linear-phase band-pass filters from narrowband up to fullband (20 Hz–20 kHz) | 16 / 32 / 48 kHz |

Detailed frequency response curves, coefficient specifications, and parameter references are available in the [Documentation](https://jr2804.github.io/g191-filter/).

---

## Documentation & Development

Full guides and references:

- [Filter Catalog & Specifications](https://jr2804.github.io/g191-filter/reference/filters/)
- [One-Shot & Batch Filtering Guide](https://jr2804.github.io/g191-filter/guides/one_shot_filtering/)
- [Blockwise & Streaming Filtering Guide](https://jr2804.github.io/g191-filter/guides/blockwise_filtering/)
- [API Reference](https://jr2804.github.io/g191-filter/reference/api/)
- [Development & Contributing](https://jr2804.github.io/g191-filter/development/)

### Installing from a non-tagged git ref

`uv add git+...` (or `pip install git+...`) runs maturin against whatever
`Cargo.toml` is at the pinned commit. The committed `Cargo.toml` placeholder
is `0.0.0`, so non-tagged installs report `g191-filter==0.0.0`. To get the
real CalVer in the wheel, either:

- Pin to a release tag: `uv add "git+https://...@v2026.9.13"`, or
- After cloning, run `uv run python scripts/install_from_git.py` which
  patches `Cargo.toml` from the most recent reachable CalVer tag and
  invokes `maturin develop --release` for you. Override with `--version
  YYYY.M.N` if there is no reachable tag.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
Coefficients and filter algorithms conform to Recommendation ITU-T G.191.
