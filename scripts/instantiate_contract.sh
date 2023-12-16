#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

code_id=$(cat $METADATA/code_id.txt)
init_msg=$(cat << EOF
{
  "assets": {
    "native_asset_denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
    "ls_asset_denom": "stk/uatom"
  }
}
EOF
)

echo "Instantiating contract..."

echo ">>> $PCORED tx wasm instantiate $code_id "$init_msg""
tx_hash=$($PCORED tx wasm instantiate $code_id "$init_msg" --from test1 --label "ica_liquid_staking" --no-admin $GAS -y | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/instantiate_tx_hash.txt

sleep 3

contract_address=$($PCORED q wasm list-contract-by-code "$code_id" -o json | jq -r '.contracts[-1]')
echo "Contract Address: $contract_address"
echo $contract_address > $METADATA/contract_address.txt
