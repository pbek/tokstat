# Use `just <recipe>` to run a recipe
# https://just.systems/man/en/

import ".shared/common.just"

# By default, run the `--list` command
default:
    @just --list

# Build a release binary with Cargo
build:
    cargo build --release

# Build a debug binary with Cargo
debug:
    cargo build

# Run the dashboard via Cargo
run args='':
    cargo run -- {{ args }}
