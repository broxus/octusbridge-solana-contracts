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
cargo-build-bpf --manifest-path=./native-proxy/Cargo.toml --bpf-out-dir=dist/program
```

#### Run tests
```bash
cargo-test-bpf --manifest-path=./token-proxy/Cargo.toml
cargo-test-bpf --manifest-path=./round-loader/Cargo.toml
cargo-test-bpf --manifest-path=./native-proxy/Cargo.toml
```

#### Build WASM bindings
```bash
wasm-pack build --target web --out-name index token-proxy -- --features wasm
wasm-pack build --target web --out-name index round-loader -- --features wasm
wasm-pack build --target web --out-name index native-proxy -- --features wasm
```

#### Build Rust bindings
```bash
cargo build --release --manifest-path=./token-proxy/Cargo.toml --features=bindings
cargo build --release --manifest-path=./round-loader/Cargo.toml --features=bindings
cargo build --release --manifest-path=./native-proxy/Cargo.toml --features=bindings
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
  --address octuswa5MD5hrTwcNBKvdxDvDQoz7C7M9sk2cRRvZfg \
  --binary dist/program/token_proxy.so \
  --url https://api.mainnet-beta.solana.com

./scripts/verify.sh \
  --address roundAsiEM445bGEp7ZwPWXUmWAHh6rpLEndJUKP1V4 \
  --binary dist/program/round_loader.so \
  --url https://api.mainnet-beta.solana.com

./scripts/verify.sh \
  --address WrapR8ncp6aGqux2TACyJh4MUxcHAHTW9eYzzeXuTJA \
  --binary dist/program/native_proxy.so \
  --url https://api.mainnet-beta.solana.com

# Leave docker container
exit
```

## Deploy
```bash
solana program deploy ./dist/program/token_proxy.so
solana program deploy ./dist/program/round_loader.so
solana program deploy ./dist/program/native_proxy.so
```

## Prepare to upgrade
```bash
solana program write-buffer --ws wss://api.mainnet-beta.solana.com dist/program/${PROGRAM_BIN}
solana program set-buffer-authority ${BUFFER_PROGRAM_ID} --new-buffer-authority ${MSIG_AUTHORITY}
```
