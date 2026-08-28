#!/bin/sh
# Build the STL reference `filter` executable with zig cc (no MSVC needed).
# Requires: scripts/clone_stl.sh already run.
set -e
STL="tmp/_stl_extract"
CC="${CC:-zig cc}"
cd "$STL/src/fir"
$CC -O2 -o ../../../filter.exe filter.c fir-dsm.c fir-flat.c fir-irs.c fir-lib.c fir-pso.c fir-tia.c fir-hirs.c fir-wb.c fir-msin.c fir-LP.c ../iir/iir-lib.c ../iir/iir-g712.c ../iir/iir-dir.c ../iir/iir-flat.c ../utl/ugst-utl.c -lm
echo "Built $STL/filter.exe"
