#!/bin/bash
# Build the comparison binary and run it on the sample SVG.
# Output PDFs and PNG previews land in ./out/.
set -euo pipefail

cd "$(dirname "$0")"

SAMPLE="${1:-samples/the_tcp_ip_protocol_stack.svg}"
OUT_DIR="out"

cargo build --release --quiet
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

./target/release/compare "$SAMPLE" "$OUT_DIR"
