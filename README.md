# Hubble

[![codecov](https://codecov.io/gh/hubble-markets/hubble/branch/master/graph/badge.svg?token=0T27B3WLJF)](https://codecov.io/gh/hubble-markets/hubble)

The smart contracts, tests and deployment configs for the Hubble Protocol.


## How to build - ensure you've set up your tooling
1. Build `$ make build`
2. For purely rust compilation checks, run `cargo check` or `cargo build`

## How to deploy - ensure it's built and env set up
- `$ make deploy` or `$ make deploy-new` if you don't want to upgrade the existing program. For the first time deployment, copy the programId and override it everywhere
- To generate & initialized the global state structs, run `npx ts-mocha tests/deployment.ts --grep initialize_protocol`
- Copy the config outputs into runtimeConfig.json (for the frontend) or in `deployment.ts`
- To generate some users (for devnet), run the tests `generate_borrowers_from_fixed_config` in deployment.ts

## How to run tests (unit & integration) - ensure it's deployed & env set up
1. Run integration tests `$ npx ts-mocha tests/tests_borrowing.ts` or `npx ts-mocha -t tests_borrowing/borrowing.ts --grep initialize_trove`
2. Run unit tests `$ cargo test` and `$ cargo test -- --ignored`
3. Generate users `$ npx ts-mocha tests/*.ts  --grep generate_borrowers`

## Run docker tests
```docker build -f integration.Dockerfile -t hubble-integrations .```

## Environment configuration
- localhost
    - `$ solana config set --url http://localhost:8899/`
    - Ensure `env` is using `localnet`
    - `$ make start-validator`
- devnet
    - `$ solana config set --url https://api.devnet.solana.com`
    - Ensure `env` is using `devnet`


## First time tooling set up
- You need rust & anchor `https://project-serum.github.io/anchor/getting-started/introduction.html`
- You need solana tools cli `https://docs.solana.com/cli/install-solana-cli-tools`


## Common Errors

- "account data too small for instruction"
    - this can happen when you have added some extra instructions to an already deployed program
    - to fix this, you need to deploy to a new address
    - could be a big problem if this is production!
    - maybe we should use "proxy" like in ethereum

## Deployment order

-> Set HBB issuance start date three days after IDO
-> Set Config correctly
-> Set min redemption amount
-> Set redemption bootstrap date

