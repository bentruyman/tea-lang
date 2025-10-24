#!/usr/bin/env bash

set -euo pipefail

cargo build --release
cp target/release/tea-cli ~/.cargo/bin/tea
tea --version
