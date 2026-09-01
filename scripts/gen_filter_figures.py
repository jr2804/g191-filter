"""Generate frequency response SVG figures for ITU-T G.191 filters using the xy package.

Saves figures directly into docs/assets/figures/.

Styling: light-gray card background (readable on the docs' dark mode), enlarged
fonts, thicker lines, and `tight_layout()` on every figure.
"""
# csort: off
# Function definition order is narrative (helpers -> generate_* -> main) and
# not meant to be reshuffled by structural sorters.

from __future__ import annotations

import math
from pathlib import Path

import numpy as np
import xy.pyplot as plt

import g191_filter as g191

OUT_DIR = Path("docs/assets/figures")
OUT_DIR.mkdir(parents=True, exist_ok=True)

# Figure card background: light gray stays readable on both light and dark pages.
BG = "#f4f4f5"

plt.rcParams.update({
    # Fonts
    "font.size": 13,
    "axes.titlesize": 16,
    "axes.labelsize": 14,
    "legend.fontsize": 12,
    "xtick.labelsize": 12,
    "ytick.labelsize": 12,
    # Backgrounds
    "figure.facecolor": BG,
    "axes.facecolor": BG,
    # Ink
    "axes.edgecolor": "#71717a",
    "axes.labelcolor": "#27272a",
    "text.color": "#27272a",
    "xtick.color": "#3f3f46",
    "ytick.color": "#3f3f46",
    "grid.color": "#d4d4d8",
    # Lines
    "lines.linewidth": 2.5,
})

# List of all 20 individual filters with their native parameters and visual styling
INDIVIDUAL_FILTERS = [
    # Resampling FIR
    {
        "id": "hq_down_2_to_1",
        "sr": 16000,
        "title": "High-Quality 2:1 Downsampling FIR (16 kHz)",
        "color": "#0284c7",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    {
        "id": "hq_down_3_to_1",
        "sr": 16000,
        "title": "High-Quality 3:1 Downsampling FIR (16 kHz)",
        "color": "#0369a1",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    # Band-Pass FIR
    {
        "id": "flat_band_pass",
        "sr": 8000,
        "title": "Flat Band-Pass FIR (0.3 - 3.4 kHz, 8 kHz)",
        "color": "#0d9488",
        "f_min": 50,
        "y_domain": (-80, 10),
    },
    # IRS Family FIR
    {
        "id": "irs8khz",
        "sr": 8000,
        "title": "IRS 8 kHz (Intermediate Reference System)",
        "color": "#2563eb",
        "f_min": 50,
        "y_domain": (-60, 10),
    },
    {
        "id": "irs16khz",
        "sr": 16000,
        "title": "IRS 16 kHz (Intermediate Reference System)",
        "color": "#3b82f6",
        "f_min": 50,
        "y_domain": (-70, 10),
    },
    {
        "id": "mod_irs16khz",
        "sr": 16000,
        "title": "Modified IRS 16 kHz (Wideband IRS)",
        "color": "#f59e0b",
        "f_min": 50,
        "y_domain": (-60, 10),
    },
    {
        "id": "mod_irs48khz",
        "sr": 48000,
        "title": "Modified IRS 48 kHz (Fullband IRS)",
        "color": "#d97706",
        "f_min": 50,
        "y_domain": (-70, 10),
    },
    # 48 kHz Low-Pass FIR Family
    {
        "id": "lp1p5_48khz",
        "sr": 48000,
        "title": "Low-Pass 1.5 kHz Filter (48 kHz)",
        "color": "#10b981",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    {
        "id": "lp35_48khz",
        "sr": 48000,
        "title": "Low-Pass 3.5 kHz Filter (48 kHz)",
        "color": "#059669",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    {
        "id": "lp7_48khz",
        "sr": 48000,
        "title": "Low-Pass 7.0 kHz Filter (48 kHz)",
        "color": "#047857",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    {
        "id": "lp10_48khz",
        "sr": 48000,
        "title": "Low-Pass 10.0 kHz Filter (48 kHz)",
        "color": "#0f766e",
        "f_min": 50,
        "y_domain": (-100, 10),
    },
    {
        "id": "lp12_48khz",
        "sr": 48000,
        "title": "Low-Pass 12.0 kHz Filter (48 kHz)",
        "color": "#115e59",
        "f_min": 50,
        "y_domain": (-90, 10),
    },
    {
        "id": "lp14_48khz",
        "sr": 48000,
        "title": "Low-Pass 14.0 kHz Filter (48 kHz)",
        "color": "#134e4a",
        "f_min": 50,
        "y_domain": (-60, 15),
    },
    {
        "id": "lp20_48khz",
        "sr": 48000,
        "title": "Low-Pass 20.0 kHz Filter (48 kHz)",
        "color": "#064e3b",
        "f_min": 50,
        "y_domain": (-50, 15),
    },
    # IIR Filters
    {
        "id": "g712_8khz",
        "sr": 8000,
        "title": "G.712 PCM Speech Filter (8 kHz)",
        "color": "#8b5cf6",
        "f_min": 50,
        "y_domain": (-60, 20),
    },
    {
        "id": "dir_dc_removal",
        "sr": 8000,
        "title": "Direct DC Removal High-Pass Filter (8 kHz)",
        "color": "#ec4899",
        "f_min": 5,
        "y_domain": (-40, 5),
    },
    {
        "id": "iir_down_3_to_1",
        "sr": 16000,
        "title": "IIR Direct 3:1 Downsampling Filter (16 kHz)",
        "color": "#6366f1",
        "f_min": 50,
        "y_domain": (-120, 10),
    },
    {
        "id": "iir_up_1_to_3",
        "sr": 16000,
        "title": "IIR Direct 1:3 Upsampling Filter (16 kHz)",
        "color": "#4f46e5",
        "f_min": 50,
        "y_domain": (-120, 10),
    },
    {
        "id": "iir_casc_lp_3_to_1",
        "sr": 48000,
        "title": "IIR Cascade 3:1 Low-Pass Filter (48 kHz)",
        "color": "#7c3aed",
        "f_min": 50,
        "y_domain": (-120, 10),
    },
    {
        "id": "iir_casc_lp_1_to_3",
        "sr": 48000,
        "title": "IIR Cascade 1:3 Low-Pass Filter (48 kHz)",
        "color": "#6d28d9",
        "f_min": 50,
        "y_domain": (-120, 15),
    },
]


# Log-frequency tick positions shared by every figure. The Nyquist frequency
# itself is never labeled; ticks below the axis minimum are dropped.
_FREQ_TICKS = [20, 100, 500, 1000, 4000, 8000, 16000, 20000]


def _apply_freq_ticks(ax: plt.Axes, sr: int) -> None:
    """Set manual Hz tick labels (1k instead of 10^3), capped below Nyquist."""
    lo = ax.get_xlim()[0]
    ticks = [t for t in _FREQ_TICKS if lo <= t < sr / 2.0]
    ax.set_xticks(ticks)
    ax.set_xticklabels([f"{t / 1000:g}k" if t >= 1000 else f"{t:g}" for t in ticks], rotation=30)


def _response(fid: str, n: int, sr: int, f_min: float) -> tuple[np.ndarray, np.ndarray]:
    """Frequency response masked to [f_min, Nyquist]."""
    w, mag = g191.get_frequency_response(fid, n, sr)
    mask = (w >= f_min) & (w <= sr / 2.0)
    return w[mask], mag[mask]


def _dc_gain_db(fid: str) -> float:
    """DC gain in dB = 20*log10(|sum(b)|) for a (typically FIR) filter."""
    b, _a = g191.get_coefficients_ba_py(fid)
    s = abs(sum(b))
    return 20 * math.log10(s) if s > 0 else 0.0


def _finalize(fig: plt.Figure, path: Path) -> None:
    fig.tight_layout()
    fig.savefig(path)
    print(f"Generated {path}")


def generate_single_charts() -> None:
    for item in INDIVIDUAL_FILTERS:
        fid = item["id"]
        sr = item["sr"]
        f_min = item["f_min"]
        w_p, mag_p = _response(fid, 2048, sr, f_min)

        fig, ax = plt.subplots(figsize=(7.2, 3.8))
        ax.semilogx(w_p, mag_p, color=item["color"])
        ax.set(
            ylabel="Magnitude (dB)",
            title=item["title"],
            xlim=(f_min, sr / 2.0),
            ylim=item["y_domain"],
        )
        ax.set_xlabel("Frequency (Hz)", labelpad=40)
        ax.grid(True, which="both", alpha=0.4)
        _apply_freq_ticks(ax, sr)
        _finalize(fig, OUT_DIR / f"{fid}.svg")


def generate_group_charts() -> None:
    # 1. IRS Family Comparison
    # Okabe-Ito colorblind-safe palette; linestyle redundancy so hue is
    # never the only channel separating series.
    fig, ax = plt.subplots(figsize=(7.4, 4.2))
    for fid, sr, lbl, col, ls in [
        ("irs8khz", 8000, "IRS 8 kHz", "#0072B2", "-"),
        ("irs16khz", 16000, "IRS 16 kHz", "#56B4E9", "--"),
        ("mod_irs16khz", 16000, "Mod IRS 16 kHz", "#E69F00", "-."),
        ("mod_irs48khz", 48000, "Mod IRS 48 kHz", "#D55E00", (0, (3, 1, 1, 1))),
    ]:
        w, m = _response(fid, 2048, sr, 50)
        ax.semilogx(w, m, color=col, linestyle=ls, label=lbl)
    ax.set(
        ylabel="Magnitude (dB)",
        title="Intermediate Reference System (IRS) Filter Family",
        xlim=(50, 24000),
        ylim=(-60, 10),
    )
    ax.set_xlabel("Frequency (Hz)", labelpad=40)
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()
    _apply_freq_ticks(ax, 48000)
    _finalize(fig, OUT_DIR / "irs_family.svg")

    # 2. 48 kHz Low-Pass FIR Family Comparison
    fig, ax = plt.subplots(figsize=(7.4, 4.2))
    for fid, lbl, col, ls in [
        ("lp1p5_48khz", "LP 1.5 kHz", "#0072B2", "-"),
        ("lp35_48khz", "LP 3.5 kHz", "#D55E00", "--"),
        ("lp7_48khz", "LP 7.0 kHz", "#009E73", "-."),
        ("lp10_48khz", "LP 10.0 kHz", "#CC79A7", (0, (3, 1, 1, 1))),
        ("lp12_48khz", "LP 12.0 kHz", "#56B4E9", (0, (5, 2))),
        ("lp14_48khz", "LP 14.0 kHz", "#E69F00", (0, (1, 1))),
        ("lp20_48khz", "LP 20.0 kHz", "#000000", (0, (5, 1, 1, 1))),
    ]:
        w, m = _response(fid, 2048, 48000, 50)
        # STL LP filters have non-unity DC gain by design; normalize so the
        # family chart shows passband shape around 0 dB. Individual charts
        # plot the unnormalized (true) response.
        m = m - _dc_gain_db(fid)
        ax.semilogx(w, m, color=col, linestyle=ls, label=lbl)
    ax.set(
        ylabel="Magnitude (dB, normalized to DC)",
        title="G.191 48 kHz Low-Pass Filter Suite (normalized to 0 dB DC)",
        xlim=(50, 24000),
        ylim=(-100, 5),
    )
    ax.set_xlabel("Frequency (Hz)", labelpad=40)
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()
    _apply_freq_ticks(ax, 48000)
    _finalize(fig, OUT_DIR / "lp_48k_family.svg")

    # 3. Resampling & Rate-Change Filters Comparison
    fig, ax = plt.subplots(figsize=(7.4, 4.2))
    for fid, sr, lbl, col, ls in [
        ("hq_down_2_to_1", 16000, "HQ Down 2:1 FIR (16 kHz)", "#0072B2", "-"),
        ("hq_down_3_to_1", 16000, "HQ Down 3:1 FIR (16 kHz)", "#56B4E9", "--"),
        ("iir_down_3_to_1", 16000, "IIR Down 3:1 (16 kHz)", "#E69F00", "-."),
        ("iir_casc_lp_3_to_1", 48000, "IIR Casc LP 3:1 (48 kHz)", "#CC79A7", (0, (3, 1, 1, 1))),
    ]:
        w, m = _response(fid, 2048, sr, 50)
        ax.semilogx(w, m, color=col, linestyle=ls, label=lbl)
    ax.set(
        ylabel="Magnitude (dB)",
        title="G.191 Rate-Conversion & Resampling Filters",
        xlim=(50, 24000),
        ylim=(-120, 10),
    )
    ax.set_xlabel("Frequency (Hz)", labelpad=40)
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()
    _apply_freq_ticks(ax, 48000)
    _finalize(fig, OUT_DIR / "resampling_family.svg")

    # 4. Telecom & Processing Filters (G.712, DC Removal, Flat Band-Pass)
    fig, ax = plt.subplots(figsize=(7.4, 4.2))
    for fid, sr, lbl, col, ls in [
        ("flat_band_pass", 8000, "Flat Band-Pass (0.3-3.4 kHz)", "#009E73", "-"),
        ("g712_8khz", 8000, "G.712 PCM Filter", "#0072B2", "--"),
        ("dir_dc_removal", 8000, "DC Removal HP", "#D55E00", "-."),
    ]:
        w, m = _response(fid, 2048, sr, 10)
        ax.semilogx(w, m, color=col, linestyle=ls, label=lbl)
    ax.set(
        ylabel="Magnitude (dB)",
        title="G.191 Telecom & Conditioning Filters (8 kHz)",
        xlim=(10, 4000),
        ylim=(-60, 20),
    )
    ax.set_xlabel("Frequency (Hz)", labelpad=40)
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()
    _apply_freq_ticks(ax, 8000)
    _finalize(fig, OUT_DIR / "telecom_family.svg")


def main() -> None:
    print("Generating individual filter response charts...")
    generate_single_charts()
    print("Generating family comparison charts...")
    generate_group_charts()
    print(f"All figures successfully created in {OUT_DIR}/")


if __name__ == "__main__":
    main()
