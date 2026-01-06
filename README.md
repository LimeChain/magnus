_The repository is very much a work in progress._

# Magnus

Magnus is a modular Solana solver aimed at unifying the components required to integrate with any of the liquidity sources deployed on Solana. This includes AMMs, Prop AMMs, and Aggregators.

The implementation aims to be discrete enough to work both as a standalone, runnable binary, and a plug-and-play framework.

The system is made up of two core components:

- [magnus](./crates/magnus/src/lib.rs) - the offchain service that aggregates account state in real-time, computes predefined strategies and sends tailored transactions (or bundles).
- [router](./crates/router/src/lib.rs) - the onchain router program that facilitates the swaps.

The currently implemented liquidity sources include:

- Humidifi
- SolfiV2
- ObricV2
- Zerofi
- TesseraV
- Goonfi
- Raydium (Constant Product)
- Raydium (Concentrated Liquidity)

Since some (..most) Proprietary AMMs are deliberately obfuscated, instead of locally storing properly deserialised state and computing swap amounts, we're directly simulating through a built-in chroot-like shell. All adapters require an implementation of an interface that abstracts away any of the exchange-specific logic, which means the liquidity sources are treated interchangeably.

Check out [pmm-sim](https://github.com/limechain/pmm-sim) if you're interested in simulating and/or benchmarking swaps across any of the proprietary AMMs.

---

The solver exposes the following endpoints used to trigger a quote or swap:

```txt
=> `/api/v1/quote`
=> `/api/v1/swap`
```

The API Server's endpoints docs can be found once the solver's been span-up, at `0:0:0:0:19000`.
