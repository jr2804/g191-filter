#!/bin/sh
# Build the STL reference binaries (filter, firdemo, basop_test) via CMake.
# Requires: scripts/clone_stl.sh already run.
# Delegates to scripts/build_stl_reference.py for full CMake build.
exec python -u scripts/build_stl_reference.py
