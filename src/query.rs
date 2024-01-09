use cosmwasm_std::{Deps, StdResult};

pub fn query_ls_config(deps: Deps) -> StdResult<crate::msg::LsConfig> {
    let ls_config = crate::state::LS_CONFIG.load(deps.storage)?;
    Ok(ls_config)
}
