#!/bin/bash

set -eu

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

METADATA=${SCRIPT_DIR}/metadata
mkdir -p $METADATA

GAIAD="gaiad"
PCORED="persistenceCore"

GAS="--gas-prices 0.025uxprt --gas auto --gas-adjustment 1.5"

$PCORED config node http://localhost:26657
$GAIAD config node http://localhost:36657

CHAIN_ID="test-core-1"
