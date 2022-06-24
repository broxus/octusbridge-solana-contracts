# Solana Bridge

#### Build
```bash
cargo build-bpf --manifest-path=./token-proxy/Cargo.toml --bpf-out-dir=dist/program
cargo build-bpf --manifest-path=./round-loader/Cargo.toml --bpf-out-dir=dist/program
```

#### Deploy
```bash
solana program deploy ./dist/program/token_proxy.so
solana program deploy ./dist/program/round_loader.so
```

#### Run tests
```bash
cargo test-bpf --manifest-path=./token-proxy/Cargo.toml
cargo test-bpf --manifest-path=./round-loader/Cargo.toml
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
