# Octusbridge Solana programs

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

#### Deploy
```bash
solana program deploy ./dist/program/token_proxy.so
solana program deploy ./dist/program/round_loader.so
```

#### Prepare to upgrade
```bash
solana program write-buffer --ws wss://api.mainnet-beta.solana.com dist/program/${PROGRAM_BIN}
solana program set-buffer-authority ${BUFFER_PROGRAM_ID} --new-buffer-authority ${MSIG_AUTHORITY}
```
