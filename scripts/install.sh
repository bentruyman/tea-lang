#!/usr/bin/env bash

set -euo pipefail

cargo build --release
cp target/release/tea-cli ~/.cargo/bin/tea
cp target/release/tea-lsp ~/.cargo/bin/tea-lsp
tea --version
