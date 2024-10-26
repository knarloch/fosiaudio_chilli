#!/bin/bash

set -ex

sudo apt install gcc-aarch64-linux-gnu

rustup -V || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-musl
