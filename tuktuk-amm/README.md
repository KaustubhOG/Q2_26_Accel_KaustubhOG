# AMM

A decentralized Automated Market Maker built on Solana using the Anchor framework. It implements the constant product formula (`x * y = k`) to allow users to swap between two tokens without an order book or centralized counterparty.

---

## What It Does

- Creates a liquidity pool for any two SPL token pairs
- Allows liquidity providers to deposit tokens and receive LP tokens representing their share
- Allows users to swap one token for another with price determined by the pool ratio
- Allows liquidity providers to withdraw their share by burning LP tokens

---

## Code Structure

```
programs/amm/src/
├── lib.rs                  program entry point, instruction routing
├── instructions/
│   ├── initialize.rs       creates the pool and mints LP token
│   ├── deposit.rs          adds liquidity, mints LP tokens to provider
│   ├── withdraw.rs         burns LP tokens, returns underlying assets
│   └── swap.rs             executes token swap using constant product formula
└── state/
    └── config.rs           pool config account storing token mints and fees
```

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation)
- [Node.js](https://nodejs.org/) (v18 or above)
- [Yarn](https://yarnpkg.com/)

---

## How to Run

```bash
cd AMM
yarn install
anchor build
anchor test
```

Make sure your Solana CLI is set to localnet:

```bash
solana config set --url localhost
```

Start a local validator if not already running:

```bash
solana-test-validator
```
