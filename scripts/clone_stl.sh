#!/bin/sh
# Clone the ITU-T STL reference code for verification (NOT committed to repo).
set -e
DEST="tmp/_stl_extract"
if [ -d "$DEST" ]; then
  echo "STL already cloned at $DEST"
  exit 0
fi
mkdir -p tmp
git clone --depth 1 --branch STL2026_ITU-T_submission https://github.com/openitu/STL.git "$DEST"
echo "Done. Reference code at $DEST"
