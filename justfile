# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# gdscheck developer tasks — run with `just <recipe>` (https://github.com/casey/just)

# List available recipes (default).
default:
    @just --list

# Run the full test suite (release, like CI).
test:
    cargo test --release

# Lint with clippy, warnings as errors.
clippy:
    cargo clippy --release -- -D warnings

# Build the release binary.
build:
    cargo build --release

# Regenerate the IHP SG13G2 test fixtures from the generators.
gen-testdata:
    cargo run --release --features dev-tools --bin gen-testdata -- --pdk pdks/ihp-sg13g2/pdk.yml

# Format, lint, and test — the pre-commit gate.
check: clippy test
    cargo fmt --check
