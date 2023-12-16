use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub assets: AssetData,
}

#[cw_serde]
pub struct LsConfig {
    /// Flag to enable/disable the contract
    pub active: Option<bool>,
}

/// holds the native and ls asset denoms relevant for providing liquidity.
#[cw_serde]
pub struct AssetData {
    pub native_asset_denom: String,
    pub ls_asset_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    LiquidStake { receiver: Addr },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(StakedLiquidityInfo)]
    GetStakedLiquidity {},
    #[returns(Vec<AssetData>)]
    Assets {},
    #[returns(LsConfig)]
    LsConfig {},
}

#[cw_serde]
pub enum MigrateMsg {
    UpdateConfig {
        assets: Option<AssetData>,
        ls_config: Option<LsConfig>,
    },
}

/// keeps track of provided asset liquidity in `Uint128`.
#[cw_serde]
pub struct StakedLiquidityInfo {
    pub staked_amount_native: Uint128,
}
