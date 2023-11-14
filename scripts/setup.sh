#!/usr/bin/env bash

# Install RUST 
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

# Install WASM
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Generate keypair
solana-keygen new --no-bip39-passphrase
