BUILDDIR ?= $(CURDIR)/build

.PHONY: build
build:
	cargo wasm

.PHONY: fmt
fmt:
  cargo fmt

build-debug:
	cargo wasm-debug

build-optimized: fmt build
	docker run --rm -v "$(CURDIR)":/code \
		--mount type=volume,source="$(notdir $(CURDIR))_cache",target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer:0.14.0

# Uploads the contract to osmosis
store-contract:
	bash scripts/store_contract.sh

# Instantiates the contract
instantiate-contract:
	bash scripts/instantiate_contract.sh

# Execute liquid staking
liquid-stake:
	bash scripts/execute_liquid_stake.sh

# Execute ibc liquid staking
ibc-liquid-stake:
	bash scripts/ibc_liquid_stake.sh

# Queries all metrics stored in the contract
query-all:
	bash scripts/query.sh
