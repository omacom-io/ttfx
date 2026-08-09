#!/usr/bin/env bash
# Fetch the pinned TerminalTextEffects reference used by the parity suites.
#
# The reference is NOT vendored into this repo — it's upstream's code, and it
# belongs to upstream. The parity harness clones it at one pinned commit so
# every comparison is against a known tree.
set -euo pipefail
cd "$(dirname "$0")/../.."

REF_REPO="https://github.com/ChrisBuilds/terminaltexteffects"
REF_COMMIT="7a91dd9ca6ee0c4f4b1484efee0ecac1bb84104e"  # v0.15.0
DEST="reference/tte"

if [ -d "$DEST/.git" ] && [ "$(git -C "$DEST" rev-parse HEAD 2>/dev/null)" = "$REF_COMMIT" ]; then
  echo "reference already at $REF_COMMIT"
  exit 0
fi

rm -rf "$DEST"
mkdir -p "$(dirname "$DEST")"
git clone -q --filter=blob:none "$REF_REPO" "$DEST"
git -C "$DEST" checkout -q "$REF_COMMIT"
echo "reference/tte pinned at $REF_COMMIT (v0.15.0)"
