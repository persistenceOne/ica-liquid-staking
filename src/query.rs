use cosmwasm_std::{Deps, StdResult};

use crate::{msg::{StakedLiquidityInfo, UnstakedLiquidityInfo}, state::{STAKED_LIQUIDITY_INFO, UNSTAKED_LIQUIDITY_INFO}};

pub fn query_staked_liquidity_info(deps: Deps) -> StdResult<StakedLiquidityInfo> {
    let staked_liquidity_info = STAKED_LIQUIDITY_INFO.load(deps.storage)?;
    Ok(staked_liquidity_info)
}

pub fn query_unstaked_liquidity_info(deps: Deps) -> StdResult<UnstakedLiquidityInfo> {
    let unstaked_liquidity_info = UNSTAKED_LIQUIDITY_INFO.load(deps.storage)?;
    Ok(unstaked_liquidity_info)
}

pub fn query_assets(deps: Deps) -> StdResult<crate::msg::AssetData> {
    let assets = crate::state::ASSETS.load(deps.storage)?;
    Ok(assets)
}

pub fn query_ls_config(deps: Deps) -> StdResult<crate::msg::LsConfig> {
    let ls_config = crate::state::LS_CONFIG.load(deps.storage)?;
    Ok(ls_config)
}
