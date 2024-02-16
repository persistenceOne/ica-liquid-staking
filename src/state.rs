use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{IbcConfig, LsConfig};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LSInfo {
    pub receiver: Addr,
    pub transfer_channel: Option<String>,
    pub ibc_denom: String,
    pub ls_token_denom: String,
    pub prev_ls_token_balance: Uint128,
    pub balance_change: Uint128,
    pub recovery_address: Option<Addr>,
}

/// configuration relevant to entering into an LS
pub const LS_CONFIG: Item<LsConfig> = Item::new("ls_config");

// Holds temp state for the ls message that the contract is currently processing
pub const CURRENT_TX: Item<LSInfo> = Item::new("current_tx");

/// config containing ibc fee, ica timeout, and ibc transfer
pub const IBC_CONFIG: Item<IbcConfig> = Item::new("ibc_config");
