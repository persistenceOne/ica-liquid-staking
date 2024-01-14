BUILDDIR ?= $(CURDIR)/build

.PHONY: build
build:
	cargo wasm

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: test
test:
	cargo test

build-debug:
	cargo wasm-debug

build-optimized: fmt build test
	docker run --rm -v "$(CURDIR)":/code \
		--mount type=volume,source="$(notdir $(CURDIR))_cache",target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/optimizer:0.15.0

# Uploads the contract to osmosis
store-contract:
	bash tests/scripts/store_contract.sh

# Instantiates the contract
instantiate-contract:
	bash tests/scripts/instantiate_contract.sh

# Execute liquid staking
liquid-stake:
	bash tests/scripts/execute_liquid_stake.sh

# Execute ibc liquid staking with ibc transfer out
ibc_liquid_stake_ibc_transfer_out:
	bash tests/scripts/ibc_liquid_stake_ibc_transfer_out.sh

# Execute ibc liquid staking with ica transfer out
ibc_liquid_stake_ica_transfer_out:
	bash tests/scripts/ibc_liquid_stake_ica_transfer_out.sh

# Queries all metrics stored in the contract
query-all:
	bash tests/scripts/query.sh

###############################################################################
###                             Interchain test                             ###
###############################################################################

# Executes IBC tests via interchaintest
ictest-ibc:
	cd tests/interchaintest && \
  go clean -testcache && \
  go test -timeout=25m -race -v -run TestPersistenceGaiaIBCTransfer .
