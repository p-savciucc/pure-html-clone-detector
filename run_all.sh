#!/bin/bash

# run_all.sh - Automated pipeline for HTML Clone Detector

# -------------------------------
# 1. Validate environment
# -------------------------------
if ! command -v node &> /dev/null; then
    echo "❌ Rust not found. Please install Rust toolchain"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Please install Rust toolchain"
    exit 1
fi

# -------------------------------
# 2. Run Node.js renderer
# -------------------------------
echo "🚀 Starting HTML-NODE-PROCESSOR rendering phase..."
cd html-node-processor/src || { echo "❌ html-node-processor not found"; exit 1; }

START_NODE_MS=$(date +%s%3N)
cargo run --release
NODE_EXIT_CODE=$?
END_NODE_MS=$(date +%s%3N)

if [ $NODE_EXIT_CODE -ne 0 ]; then
    echo "❌ main.rs renderer failed with code $NODE_EXIT_CODE"
    exit 1
fi

echo ""

# -------------------------------
# 3. Run Rust clustering
# -------------------------------
echo "🦀 Starting Rust clustering phase..."
cd ../../rust-core/src || { echo "❌ rust-core directory not found"; exit 1; }

START_RUST_MS=$(date +%s%3N)
cargo run --release
RUST_EXIT_CODE=$?
END_RUST_MS=$(date +%s%3N)

if [ $RUST_EXIT_CODE -ne 0 ]; then
    echo "❌ Rust clustering failed with code $RUST_EXIT_CODE"
    exit 1
fi

# -------------------------------
# 4. Final output
# -------------------------------
TOTAL_MS=$((END_RUST_MS - START_NODE_MS))
TOTAL_SEC=$((TOTAL_MS / 1000))
TOTAL_REST_MS=$((TOTAL_MS % 1000))

echo ""
echo "🎉 Total processing time: ${TOTAL_SEC} s ${TOTAL_REST_MS} ms"
echo "📊 Results available in:"
echo "   - node-renderer/output/ (rendered data)"
echo "   - rust-core/output/ (cluster analysis)"
