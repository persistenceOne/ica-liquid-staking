#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

CONTRACT=./artifacts/ica_liquid_staking.wasm

echo "Storing contract..."

echo ">>> $PCORED tx wasm store $CONTRACT"
tx=$($PCORED tx wasm store $CONTRACT $GAS --from test1 -y)
echo $tx
tx_hash=$(echo $tx | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/store_tx_hash.txt

sleep 3

code_id=$($PCORED q tx $tx_hash | jq -r '.logs[0].events[-1].attributes[-1].value')
echo "Code ID: $code_id"
echo $code_id > $METADATA/code_id.txt
