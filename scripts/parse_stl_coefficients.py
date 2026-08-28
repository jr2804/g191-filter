#!/usr/bin/env python3
"""
Parse STL C reference code and extract filter coefficients.
Outputs Rust arrays with normalized float values.
"""

import pathlib
import re
from collections import OrderedDict

BASE = pathlib.Path(__file__).resolve().parents[1]
STL_DIR = BASE / "tmp" / "_stl_extract" / "src"

# F24 scale factor = 0x00800000
F24 = 0x00800000
# F16 scale factor = 32768.0
F16 = 32768.0


def generate_rust_coefficients() -> None:
    """Generate Rust coefficient arrays from parsed C files."""
    # Read G.191 reference for filter parameters
    print("Parsing STL C reference code...")

    fir_filters = get_fir_filters()
    iir_filters = get_iir_filters()

    print(f"Found {len(fir_filters)} FIR filters and {len(iir_filters)} IIR filters")

    for name, info in fir_filters.items():
        print(f"  FIR: {name} - {info['len']} coeffs, sr={info['sr']}, ratio={info['ratio']}")

    for name, info in iir_filters.items():
        if info["type"] == "parallel":
            print(f"  IIR (parallel): {name} - sr={info['sr']}, {len(info['b'])} blocks")
        else:
            print(f"  IIR (direct): {name} - sr={info['sr']}, b={info['b']}, a={info['a']}")


def get_fir_filters():
    """Parse all FIR filter files."""
    filters = OrderedDict()

    # FIR flat filters (fir-flat.c)
    flat = parse_coefficients(STL_DIR / "fir" / "fir-flat.c", None)
    if "h02" in flat:
        filters["hq_down_2_to_1"] = {
            "values": flat["h02"]["values"],
            "len": flat["h02"]["len"],
            "sr": 16000,
            "ratio": (1, 2),
            "gain": 1.0,
        }
    if "h03" in flat:
        filters["hq_down_3_to_1"] = {
            "values": flat["h03"]["values"],
            "len": flat["h03"]["len"],
            "sr": 16000,
            "ratio": (1, 3),
            "gain": 1.0,
        }
    if "flat_coef" in flat:
        filters["flat_band_pass"] = {
            "values": flat["flat_coef"]["values"],
            "len": flat["flat_coef"]["len"],
            "sr": 48000,
            "ratio": (1, 1),
            "gain": 1.0,
        }

    # FIR IRS filters (fir-irs.c)
    irs = parse_coefficients(STL_DIR / "fir" / "fir-irs.c", None)
    for name in ["h0IRS8", "h0IRS16", "mod_irs16_coef", "mod_irs48_coef"]:
        if name in irs:
            filters[name.lower()] = {
                "values": irs[name]["values"],
                "len": irs[name]["len"],
                "sr": 8000 if "8" in name.lower() and "16" not in name.lower() else 16000,
                "ratio": (1, 1),
                "gain": 1.0,
            }

    return filters


def parse_coefficients(filepath, var_pattern, value_pattern=None):
    """Parse a C file and extract coefficient arrays."""
    content = filepath.read_text(encoding="utf-8", errors="replace")

    # Find all static float arrays
    pattern = r"static\s+(?:float|double)\s+(\w+)\[(\w+)\]\s*=\s*\{([^}]+)\}"
    arrays = re.findall(pattern, content, re.DOTALL)

    results = {}
    for name, dim, raw_values in arrays:
        # Determine scale factor
        values = []
        for line in raw_values.strip().split("\n"):
            line = line.strip()
            if not line or line.startswith("//") or line.startswith("/*"):
                continue
            # Split by comma
            for val_str in line.split(","):
                val_str = val_str.strip()
                if not val_str:
                    continue
                val_str = val_str.replace("f24", str(F24)).replace("f16", str(F16))
                # Evaluate the expression (e.g., "1584. / f24")
                try:
                    # Handle scientific notation and expressions
                    val = float(eval(val_str.replace("/ f24", f"/ {F24}").replace("/ f16", f"/ {F16}")))
                    values.append(val)
                except Exception:
                    pass

        results[name] = {
            "values": values,
            "dim": dim,
            "len": len(values),
        }

    return results


def get_iir_filters():
    """Parse all IIR filter files."""
    filters = {}

    # G.712 parallel-form filters (iir-g712.c)
    g712_content = (STL_DIR / "iir" / "iir-g712.c").read_text(encoding="utf-8", errors="replace")

    # b_16khz[4][3] array
    b_match = re.search(r"b_16khz\[4\]\[3\]\s*=\s*\{([^}]+)\}", g712_content, re.DOTALL)
    c_match = re.search(r"c_16khz\[4\]\[2\]\s*=\s*\{([^}]+)\}", g712_content, re.DOTALL)

    if b_match and c_match:
        b_raw = b_match.group(1)
        c_raw = c_match.group(1)

        b_rows = re.findall(r"\{[^}]+\}", b_raw)
        c_rows = re.findall(r"\{[^}]+\}", c_raw)

        b_vals = []
        for row in b_rows:
            vals = re.findall(r"(F24|[\d.]+)\s*/\s*f24", row)
            if vals:
                b_vals.append([float(v) / F24 if v != "F24" else 1.0 for v in vals])
            else:
                vals = re.findall(r"[\d.]+", row)
                b_vals.append([float(v) / F24 for v in vals])

        c_vals = []
        for row in c_rows:
            vals = re.findall(r"(F24|[\d.]+)\s*/\s*f24", row)
            if vals:
                c_vals.append([float(v) / F24 if v != "F24" else 1.0 for v in vals])
            else:
                vals = re.findall(r"[\d.]+", row)
                c_vals.append([float(v) / F24 for v in vals])

        filters["g712_8khz"] = {
            "type": "parallel",
            "gain": 1.0,
            "direct": 0.0,
            "b": b_vals,
            "c": c_vals,
            "sr": 8000,
        }

    # DC removal filter (iir-dir.c)
    (STL_DIR / "iir" / "iir-dir.c").read_text(encoding="utf-8", errors="replace")
    b_dc = [1.0, -1.0]
    a_dc = [1.0, -0.985]

    filters["dir_dc_removal"] = {
        "type": "direct",
        "gain": 1.0,
        "b": b_dc,
        "a": a_dc,
        "sr": 48000,
    }

    return filters


if __name__ == "__main__":
    generate_rust_coefficients()
