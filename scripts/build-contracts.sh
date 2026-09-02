#!/bin/bash
set -e

echo "Building ChainBounty contracts..."

# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM (requires soroban-cli with opt features)
echo "Optimizing WASM..."
soroban contract optimize \
  --wasm target/wasm32-unknown-unknown/release/chainbounty.wasm \
  --wasm-out target/wasm32-unknown-unknown/release/chainbounty_optimized.wasm

echo "✓ Build complete"
echo "  Optimized WASM: target/wasm32-unknown-unknown/release/chainbounty_optimized.wasm"
