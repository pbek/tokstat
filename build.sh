#!/usr/bin/env bash
set -euo pipefail

echo "Building tokstat..."

# Check if nix is available
if command -v nix &>/dev/null; then
  echo "Using Nix build..."
  nix build
  echo "✓ Build complete! Binary at: ./result/bin/tokstat"
elif command -v cargo &>/dev/null; then
  echo "Using Cargo build..."
  cargo build --release
  echo "✓ Build complete! Binary at: ./target/release/tokstat"
else
  echo "Error: Neither Nix nor Cargo found. Please install one of them."
  exit 1
fi
