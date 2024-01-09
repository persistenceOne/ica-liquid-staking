# ICA Liquid Staking

## Development

### Dependencies

- Rust v1.44.1+
- `wasm32-unknown-unknown` target
- Docker

### Envrionment Setup

- Install `rustup` via https://rustup.rs/
- Add `wasm32-unknown-unknown` target

    ```bash
    rustup default stable
    rustup target add wasm32-unknown-unknown
    ```

- Compile contracts, test them and generate wasm builds. Make sure the current working directory is set to the root directory of this repository, then

    ```bash
    cargo build
    cargo test
    docker run --rm -v "$(pwd)":/code \
      --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
      --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
      cosmwasm/rust-optimizer:0.14.0
    ```
