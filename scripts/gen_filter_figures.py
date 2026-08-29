"""Generate frequency response SVG figures for ITU-T G.191 filters using the xy package.

Saves figures directly into docs/assets/figures/.
"""

from __future__ import annotations

import os
from pathlib import Path
import numpy as np
import xy

import g191_filter as g191

OUT_DIR = Path("docs/assets/figures")
OUT_DIR.mkdir(parents=True, exist_ok=True)

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


def generate_single_charts() -> None:
    for item in INDIVIDUAL_FILTERS:
        fid = item["id"]
        sr = item["sr"]
        title = item["title"]
        color = item["color"]
        f_min = item["f_min"]
        y_domain = item["y_domain"]

        w, mag = g191.get_frequency_response(fid, 2048, sr)
        mask = (w >= f_min) & (w <= sr / 2.0)
        w_p = w[mask]
        mag_p = mag[mask]

        chart = xy.line_chart(
            xy.line(w_p, mag_p, name=fid, color=color, width=2.0),
            xy.x_axis(label="Frequency (Hz)", type_="log", domain=(f_min, sr / 2.0)),
            xy.y_axis(label="Magnitude (dB)", domain=y_domain),
            title=title,
            width=720,
            height=380,
        )
        out_path = OUT_DIR / f"{fid}.svg"
        xy.write_images([chart], [str(out_path)])
        print(f"Generated {out_path}")


def generate_group_charts() -> None:
    # 1. IRS Family Comparison
    w1, m1 = g191.get_frequency_response("irs8khz", 2048, 8000)
    w2, m2 = g191.get_frequency_response("irs16khz", 2048, 16000)
    w3, m3 = g191.get_frequency_response("mod_irs16khz", 2048, 16000)
    w4, m4 = g191.get_frequency_response("mod_irs48khz", 2048, 48000)

    irs_chart = xy.line_chart(
        xy.line(w1[w1 >= 50], m1[w1 >= 50], name="IRS 8 kHz", color="#2563eb", width=2.0),
        xy.line(w2[w2 >= 50], m2[w2 >= 50], name="IRS 16 kHz", color="#0284c7", width=2.0),
        xy.line(w3[w3 >= 50], m3[w3 >= 50], name="Mod IRS 16 kHz", color="#f59e0b", width=2.0),
        xy.line(w4[w4 >= 50], m4[w4 >= 50], name="Mod IRS 48 kHz", color="#ef4444", width=2.0),
        xy.x_axis(label="Frequency (Hz)", type_="log", domain=(50, 24000)),
        xy.y_axis(label="Magnitude (dB)", domain=(-60, 10)),
        title="Intermediate Reference System (IRS) Filter Family",
        width=740,
        height=420,
    )
    xy.write_images([irs_chart], [str(OUT_DIR / "irs_family.svg")])
    print(f"Generated {OUT_DIR / 'irs_family.svg'}")

    # 2. 48 kHz Low-Pass FIR Family Comparison
    lp_filters = [
        ("lp1p5_48khz", "LP 1.5 kHz", "#0284c7"),
        ("lp35_48khz", "LP 3.5 kHz", "#10b981"),
        ("lp7_48khz", "LP 7.0 kHz", "#f59e0b"),
        ("lp10_48khz", "LP 10.0 kHz", "#ef4444"),
        ("lp12_48khz", "LP 12.0 kHz", "#8b5cf6"),
        ("lp14_48khz", "LP 14.0 kHz", "#ec4899"),
        ("lp20_48khz", "LP 20.0 kHz", "#64748b"),
    ]
    lp_lines = []
    for fid, lbl, col in lp_filters:
        w, m = g191.get_frequency_response(fid, 2048, 48000)
        mask = w >= 50
        lp_lines.append(xy.line(w[mask], m[mask], name=lbl, color=col, width=1.8))

    lp_chart = xy.line_chart(
        *lp_lines,
        xy.x_axis(label="Frequency (Hz)", type_="log", domain=(50, 24000)),
        xy.y_axis(label="Magnitude (dB)", domain=(-100, 15)),
        title="G.191 48 kHz Low-Pass Filter Suite",
        width=740,
        height=420,
    )
    xy.write_images([lp_chart], [str(OUT_DIR / "lp_48k_family.svg")])
    print(f"Generated {OUT_DIR / 'lp_48k_family.svg'}")

    # 3. Resampling & Rate-Change Filters Comparison
    resamp_filters = [
        ("hq_down_2_to_1", 16000, "HQ Down 2:1 FIR (16 kHz)", "#0284c7"),
        ("hq_down_3_to_1", 16000, "HQ Down 3:1 FIR (16 kHz)", "#0ea5e9"),
        ("iir_down_3_to_1", 16000, "IIR Down 3:1 (16 kHz)", "#f59e0b"),
        ("iir_casc_lp_3_to_1", 48000, "IIR Casc LP 3:1 (48 kHz)", "#8b5cf6"),
    ]
    resamp_lines = []
    for fid, sr, lbl, col in resamp_filters:
        w, m = g191.get_frequency_response(fid, 2048, sr)
        mask = (w >= 50) & (w <= sr / 2.0)
        resamp_lines.append(xy.line(w[mask], m[mask], name=lbl, color=col, width=1.8))

    resamp_chart = xy.line_chart(
        *resamp_lines,
        xy.x_axis(label="Frequency (Hz)", type_="log", domain=(50, 24000)),
        xy.y_axis(label="Magnitude (dB)", domain=(-120, 10)),
        title="G.191 Rate-Conversion & Resampling Filters",
        width=740,
        height=420,
    )
    xy.write_images([resamp_chart], [str(OUT_DIR / "resampling_family.svg")])
    print(f"Generated {OUT_DIR / 'resampling_family.svg'}")

    # 4. Telecom & Processing Filters (G.712, DC Removal, Flat Band-Pass)
    tele_filters = [
        ("flat_band_pass", 8000, "Flat Band-Pass (0.3-3.4 kHz)", "#0d9488"),
        ("g712_8khz", 8000, "G.712 PCM Filter", "#8b5cf6"),
        ("dir_dc_removal", 8000, "DC Removal HP", "#ec4899"),
    ]
    tele_lines = []
    for fid, sr, lbl, col in tele_filters:
        w, m = g191.get_frequency_response(fid, 2048, sr)
        mask = (w >= 10) & (w <= sr / 2.0)
        tele_lines.append(xy.line(w[mask], m[mask], name=lbl, color=col, width=2.0))

    tele_chart = xy.line_chart(
        *tele_lines,
        xy.x_axis(label="Frequency (Hz)", type_="log", domain=(10, 4000)),
        xy.y_axis(label="Magnitude (dB)", domain=(-60, 20)),
        title="G.191 Telecom & Conditioning Filters (8 kHz)",
        width=740,
        height=420,
    )
    xy.write_images([tele_chart], [str(OUT_DIR / "telecom_family.svg")])
    print(f"Generated {OUT_DIR / 'telecom_family.svg'}")


def main() -> None:
    print("Generating individual filter response charts...")
    generate_single_charts()
    print("Generating family comparison charts...")
    generate_group_charts()
    print("All figures successfully created in docs/assets/figures/")


if __name__ == "__main__":
    main()
