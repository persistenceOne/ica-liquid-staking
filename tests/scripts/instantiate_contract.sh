#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

code_id=$(cat $METADATA/code_id.txt)
init_msg=$(cat << EOF
{
  "ls_prefix": "stk/",
  "timeouts": {
    "ibc_transfer_timeout": "5",
    "ica_timeout": "10"
  }
}
EOF
)

echo "Instantiating contract..."

echo ">>> $PCORED tx wasm instantiate $code_id $init_msg"
tx_hash=$($PCORED tx wasm instantiate $code_id "$init_msg" --from $USER --label "ica_liquid_staking" --no-admin $GAS -y --chain-id $CHAIN_ID | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/instantiate_tx_hash.txt

sleep 10

contract_address=$($PCORED q wasm list-contract-by-code "$code_id" -o json | jq -r '.contracts[-1]')
echo "Contract Address: $contract_address"
echo $contract_address > $METADATA/contract_address.txt
