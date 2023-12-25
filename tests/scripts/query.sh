#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

contract_address=$(cat $METADATA/contract_address.txt)

echo "Staked Liquidity:"
msg='{ "get_staked_liquidity" : { } }'
echo ">>> $PCORED q wasm contract-state smart $contract_address $msg"
$PCORED q wasm contract-state smart $contract_address "$msg"

echo "Asset:"
msg='{ "assets" : { } }'
echo ">>> $PCORED q wasm contract-state smart $contract_address $msg"
$PCORED q wasm contract-state smart $contract_address "$msg"

echo "Config:"
msg='{ "ls_config" : { } }'
echo ">>> $PCORED q wasm contract-state smart $contract_address $msg"
$PCORED q wasm contract-state smart $contract_address "$msg"
