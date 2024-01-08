use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{AssetData, LsConfig, StakedLiquidityInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LSInfo {
    pub receiver: Addr,
    pub prev_ls_token_balance: Uint128,
}

/// native and ls asset denom information
pub const ASSETS: Item<AssetData> = Item::new("assets");

/// keeps track of token amounts we staked to the pool
pub const STAKED_LIQUIDITY_INFO: Item<StakedLiquidityInfo> = Item::new("staked_liquidity_info");

/// configuration relevant to entering into an LS
pub const LS_CONFIG: Item<LsConfig> = Item::new("ls_config");

// Holds temp state for the ls message that the contract is currently processing
pub const CURRENT_TX: Item<LSInfo> = Item::new("current_tx");
