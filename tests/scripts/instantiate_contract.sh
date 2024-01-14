#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

code_id=$(cat $METADATA/code_id.txt)
init_msg=$(cat << EOF
{
  "ls_prefix": "stk/",
  "preset_ibc_fee": {
    "ack_fee": "1000",
    "timeout_fee": "1000"
  },
  "timeouts": {
    "ibc_transfer_timeout": "5",
    "ica_timeout": "10"
  }
}
EOF
)

echo "Instantiating contract..."

echo ">>> $PCORED tx wasm instantiate $code_id $init_msg"
tx_hash=$($PCORED tx wasm instantiate $code_id "$init_msg" --from test1 --label "ica_liquid_staking" --no-admin $GAS -y | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/instantiate_tx_hash.txt

sleep 3

contract_address=$($PCORED q wasm list-contract-by-code "$code_id" -o json | jq -r '.contracts[-1]')
echo "Contract Address: $contract_address"
echo $contract_address > $METADATA/contract_address.txt
