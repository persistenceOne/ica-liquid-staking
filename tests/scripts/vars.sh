#!/bin/bash

set -eu

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

METADATA=${SCRIPT_DIR}/metadata
mkdir -p $METADATA

GAIAD="gaiad"
PCORED="persistenceCore"
OSMOSISD="osmosisd"

GAS="--gas-prices 0.025uxprt --gas auto --gas-adjustment 1.5"

$PCORED config node https://rpc.testnet2.persistence.one:443
$GAIAD config node https://cosmos-testnet-rpc.polkachu.com:443
$OSMOSISD config node https://osmosis-testnet-rpc.polkachu.com:443

CHAIN_ID="test-core-2"

USER="test2 --keyring-backend test"
