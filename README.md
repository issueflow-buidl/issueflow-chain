# issueflow-contracts

## Soroban Bounty Contract Deployment (Stellar Testnet)

This repository contains the Rust/Soroban bounty contract under `contracts/bounty`.

### Deployed Contract

- Network: Stellar Testnet
- Contract ID: `CDST3TZ2XMOKTKUWIZNVOFKBKSHEDAPCFRXFQTKU6COK6FICIUTO6COD`
- Deploy date (UTC): `2026-05-25`
- Deployment transactions:
  - WASM upload: `057144fc5bd581bb296abe4170dd1fd2c96d81368c222d5daba4fb2e1074cdc0`
  - Contract deploy: `2734e045e7081d9277ab3ee2fba5d57a1bfb6318eb36785896234e0c3667182e`

Explorer links:

- https://stellar.expert/explorer/testnet/tx/057144fc5bd581bb296abe4170dd1fd2c96d81368c222d5daba4fb2e1074cdc0
- https://stellar.expert/explorer/testnet/tx/2734e045e7081d9277ab3ee2fba5d57a1bfb6318eb36785896234e0c3667182e
- https://lab.stellar.org/r/testnet/contract/CDST3TZ2XMOKTKUWIZNVOFKBKSHEDAPCFRXFQTKU6COK6FICIUTO6COD

### Reproduce Deployment

1. Install tooling:

```bash
brew install stellar-cli
rustup target add wasm32v1-none
```

2. Build and test contract:

```bash
cd contracts/bounty
cargo test
stellar contract build
```

3. Configure testnet and fund a deployer identity:

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase 'Test SDF Network ; September 2015'

stellar keys generate deployer --network testnet --fund
stellar keys address deployer
```

4. Deploy contract:

```bash
stellar contract deploy \
  --wasm ../../target/wasm32v1-none/release/bounty.wasm \
  --source deployer \
  --network testnet
```
