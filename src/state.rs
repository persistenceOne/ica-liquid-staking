use cw_storage_plus::Item;

use crate::msg::{AssetData, LsConfig, StakedLiquidityInfo};

/// native and ls asset denom information
pub const ASSETS: Item<AssetData> = Item::new("assets");

/// keeps track of token amounts we staked to the pool
pub const STAKED_LIQUIDITY_INFO: Item<StakedLiquidityInfo> = Item::new("staked_liquidity_info");

/// configuration relevant to entering into an LS
pub const LS_CONFIG: Item<LsConfig> = Item::new("ls_config");
