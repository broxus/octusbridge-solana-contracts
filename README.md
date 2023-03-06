<p align="center">
  <a href="https://github.com/venom-blockchain/developer-program">
    <img src="https://raw.githubusercontent.com/venom-blockchain/developer-program/main/vf-dev-program.png" alt="Logo" width="366.8" height="146.4">
  </a>
</p>

# Octusbridge Solana programs

## Native configuration

#### Build
```bash
cargo-build-bpf --manifest-path=./token-proxy/Cargo.toml --bpf-out-dir=dist/program
cargo-build-bpf --manifest-path=./round-loader/Cargo.toml --bpf-out-dir=dist/program
```

#### Run tests
```bash
cargo-test-bpf --manifest-path=./token-proxy/Cargo.toml
cargo-test-bpf --manifest-path=./round-loader/Cargo.toml
```

#### Build WASM bindings
```bash
wasm-pack build --target web --out-name index token-proxy -- --features wasm
wasm-pack build --target web --out-name index round-loader -- --features wasm
```

#### Build Rust bindings
```bash
cargo build --release --manifest-path=./token-proxy/Cargo.toml --features=bindings
cargo build --release --manifest-path=./round-loader/Cargo.toml --features=bindings
```

## Docker Configuration

#### Build docker container
```bash
docker build -t contract-builder .
```

#### Build contracts
```bash
# Run docker container
docker run --volume ${PWD}:/root/contracts -it --rm contract-builder:latest

# Build solana programs
./scripts/build.sh --programs

# Build WASM binding
./scripts/build.sh --wasm

# Build Rust binding
./scripts/build.sh --bindings

# Run tests
./scripts/build.sh --tests

# Verify solana programs
./scripts/verify.sh \
  --address octusZw3Ze7BRCT7C4wW4nfiwz6WGZBwUsK1HEPtZpz \
  --binary dist/program/token_proxy.so \
  --url https://api.mainnet-beta.solana.com

./scripts/verify.sh \
  --address RLoadKXJz5Nsj4YW6mefe1eNVdFUsZvxyinir7fpEeM \
  --binary dist/program/round_loader.so \
  --url https://api.mainnet-beta.solana.com

# Leave docker container
exit
```

## Deploy
```bash
solana program deploy ./dist/program/token_proxy.so
solana program deploy ./dist/program/round_loader.so
```

## Prepare to upgrade
```bash
solana program write-buffer --ws wss://api.mainnet-beta.solana.com dist/program/${PROGRAM_BIN}
solana program set-buffer-authority ${BUFFER_PROGRAM_ID} --new-buffer-authority ${MSIG_AUTHORITY}
```
