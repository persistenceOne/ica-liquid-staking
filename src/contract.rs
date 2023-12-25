#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    execute,
    msg::{ExecuteMsg, InstantiateMsg, LsConfig, MigrateMsg, QueryMsg, StakedLiquidityInfo},
    query,
    state::{ASSETS, LS_CONFIG, STAKED_LIQUIDITY_INFO},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ica-liquid-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: ls instantiate");
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ASSETS.save(deps.storage, &msg.assets)?;

    let ls_config = LsConfig { active: true };
    LS_CONFIG.save(deps.storage, &ls_config)?;

    // we begin with no liquidity staked
    STAKED_LIQUIDITY_INFO.save(
        deps.storage,
        &&StakedLiquidityInfo {
            staked_amount_native: Uint128::zero(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("active", "true")
        .add_attribute("ls_asset_denom", msg.assets.ls_asset_denom)
        .add_attribute("native_asset_denom", msg.assets.native_asset_denom))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::LiquidStake { receiver } => {
            execute::try_liquid_staking(deps, env, info, receiver)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStakedLiquidity {} => {
            to_json_binary(&query::query_staked_liquidity_info(deps)?)
        }
        QueryMsg::Assets {} => to_json_binary(&query::query_assets(deps)?),
        QueryMsg::LsConfig {} => to_json_binary(&query::query_ls_config(deps)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: migrate");

    match msg {
        MigrateMsg::UpdateConfig { assets, ls_config } => {
            let mut response = Response::default().add_attribute("method", "update_config");

            if let Some(denoms) = assets {
                ASSETS.save(deps.storage, &denoms)?;
                response = response.add_attribute("ls_denom", denoms.ls_asset_denom.to_string());
                response =
                    response.add_attribute("native_denom", denoms.native_asset_denom.to_string());
            }

            if let Some(config) = ls_config {
                LS_CONFIG.save(deps.storage, &config)?;
                response = response.add_attribute("active", config.active.to_string());
            }

            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::msg::AssetData;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json, Addr, BankMsg, CosmosMsg, ReplyOn, SubMsg};
    use persistence_std::types::cosmos::base::v1beta1::Coin as StdCoin;
    use persistence_std::types::pstake::liquidstakeibc::v1beta1::MsgLiquidStake;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            assets: AssetData {
                native_asset_denom: "earth".to_string(),
                ls_asset_denom: "stk/earth".to_string(),
            },
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LsConfig {}).unwrap();
        let value: LsConfig = from_json(&res).unwrap();
        assert_eq!(true, value.active);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Assets {}).unwrap();
        let value: AssetData = from_json(&res).unwrap();
        assert_eq!("earth", value.native_asset_denom);
        assert_eq!("stk/earth", value.ls_asset_denom);
    }

    #[test]
    fn liquid_stake() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            assets: AssetData {
                native_asset_denom: "token".to_string(),
                ls_asset_denom: "stk/token".to_string(),
            },
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::LiquidStake {
            receiver: Addr::unchecked("receiver"),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(2, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 0,
                msg: CosmosMsg::Stargate {
                    type_url: "/pstake.liquidstakeibc.v1beta1.MsgLiquidStake".to_string(),
                    value: MsgLiquidStake {
                        amount: Some(StdCoin {
                            denom: "token".to_string(),
                            amount: "2".to_string(),
                        }),
                        delegator_address: "cosmos2contract".to_string(),
                    }
                    .into(),
                },
                gas_limit: None,
                reply_on: ReplyOn::Never
            }
        );
        assert_eq!(
            res.messages[1],
            SubMsg {
                id: 0,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "receiver".to_string(),
                    amount: coins(2, "stk/token"),
                })
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Never,
            }
        );

        // ensure we can query the staked amount
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetStakedLiquidity {}).unwrap();
        let value: StakedLiquidityInfo = from_json(&res).unwrap();
        assert_eq!(2, value.staked_amount_native.u128());
    }
}
