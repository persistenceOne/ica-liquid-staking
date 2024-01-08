#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    execute::{try_liquid_staking, LS_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg, LsConfig, QueryMsg, StakedLiquidityInfo},
    query,
    reply::handle_ls_reply,
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

    let ls_config = LsConfig {
        active: true,
        chain_id: msg.chain_id,
    };
    LS_CONFIG.save(deps.storage, &ls_config)?;

    // we begin with no liquidity staked
    STAKED_LIQUIDITY_INFO.save(
        deps.storage,
        &StakedLiquidityInfo {
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
        ExecuteMsg::LiquidStake { receiver } => try_liquid_staking(deps, env, info, receiver),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        LS_REPLY_ID => handle_ls_reply(deps, env, msg),
        _ => Err(ContractError::UnknownReplyId {}),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use crate::execute::LIQUIDSTAKEIBC_RATE_QUERY_TYPE;
    use crate::msg::AssetData;
    use crate::state::{LSInfo, CURRENT_TX};

    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
    use cosmwasm_std::{
        attr, coins, from_json, Addr, BalanceResponse, BankMsg, BankQuery, Coin, ContractResult,
        CosmosMsg, Decimal, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, ReplyOn,
        SubMsg, SubMsgResponse, SystemError, SystemResult,
    };
    use persistence_std::types::cosmos::base::v1beta1::Coin as StdCoin;
    use persistence_std::types::pstake::liquidstakeibc::v1beta1::{
        MsgLiquidStake, QueryExchangeRateRequest, QueryExchangeRateResponse,
    };

    const MOCK_CHAIN_ID: &str = "chain-1";
    const NATIVE_DENOM: &str = "token";
    const LIQUIDSTAKE_DENOM: &str = "stk/token";

    pub struct WasmMockQuerier {
        pub exchange_rate: HashMap<String, QueryExchangeRateResponse>,
    }

    // Implements the Querier trait to be used as a MockQuery object
    impl Querier for WasmMockQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<Empty> = match from_json(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return SystemResult::Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                }
            };
            self.handle_query(&request)
        }
    }

    impl WasmMockQuerier {
        pub fn new() -> Self {
            WasmMockQuerier {
                exchange_rate: HashMap::new(),
            }
        }

        fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match request {
                QueryRequest::Stargate { path, data: _ } => {
                    if path == LIQUIDSTAKEIBC_RATE_QUERY_TYPE {
                        let exchange_rate_request: QueryExchangeRateRequest =
                            QueryExchangeRateRequest {
                                chain_id: MOCK_CHAIN_ID.to_string(),
                            };
                        match self.exchange_rate.get(&exchange_rate_request.chain_id) {
                            Some(resp) => SystemResult::Ok(to_json_binary(&resp).into()),
                            None => SystemResult::Err(SystemError::Unknown {}),
                        }
                    } else {
                        panic!("Mocked query not supported for stargate path {}", path);
                    }
                }
                QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
                    if address == &Addr::unchecked("cosmos2contract")
                        && denom == &LIQUIDSTAKE_DENOM.to_string()
                    {
                        let bank_res = BalanceResponse {
                            amount: Coin {
                                amount: Uint128::new(2000u128),
                                denom: denom.to_string(),
                            },
                        };
                        SystemResult::Ok(ContractResult::from(to_json_binary(&bank_res)))
                    } else {
                        unimplemented!()
                    }
                }
                _ => panic!("DO NOT ENTER HERE"),
            }
        }

        pub fn mock_exchange_rate(&mut self, chain_id: String, rate: String) {
            self.exchange_rate
                .insert(chain_id, QueryExchangeRateResponse { rate });
        }
    }

    // Helper function to instantiate the contract
    fn default_instantiate() -> (
        OwnedDeps<MockStorage, MockApi, WasmMockQuerier, Empty>,
        Env,
        MessageInfo,
    ) {
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: Default::default(),
        };

        let msg = InstantiateMsg {
            assets: AssetData {
                native_asset_denom: NATIVE_DENOM.to_string(),
                ls_asset_denom: LIQUIDSTAKE_DENOM.to_string(),
            },
            chain_id: MOCK_CHAIN_ID.to_string(),
        };

        let resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            resp.attributes,
            vec![
                attr("method", "instantiate"),
                attr("owner", "creator"),
                attr("active", "true"),
                attr("ls_asset_denom", LIQUIDSTAKE_DENOM),
                attr("native_asset_denom", NATIVE_DENOM),
            ]
        );

        (deps, env, info)
    }

    #[test]
    fn proper_initialization() {
        let (deps, _env, _info) = default_instantiate();

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LsConfig {}).unwrap();
        let value: LsConfig = from_json(&res).unwrap();
        assert_eq!(true, value.active);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Assets {}).unwrap();
        let value: AssetData = from_json(&res).unwrap();
        assert_eq!(NATIVE_DENOM, value.native_asset_denom);
        assert_eq!(LIQUIDSTAKE_DENOM, value.ls_asset_denom);
    }

    #[test]
    fn liquid_stake() {
        let (mut deps, _env, _info) = default_instantiate();

        let deposit_amount = Uint128::from(2000u128);
        let exchange_rate = "0.825537496882794638";

        // Mock each pool in the querier
        deps.querier
            .mock_exchange_rate(MOCK_CHAIN_ID.to_string(), exchange_rate.to_string());

        // beneficiary can release it
        let info = mock_info("anyone", &coins(deposit_amount.into(), "token"));
        let msg = ExecuteMsg::LiquidStake {
            receiver: Addr::unchecked("receiver"),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 1,
                msg: CosmosMsg::Stargate {
                    type_url: "/pstake.liquidstakeibc.v1beta1.MsgLiquidStake".to_string(),
                    value: MsgLiquidStake {
                        amount: Some(StdCoin {
                            denom: NATIVE_DENOM.to_string(),
                            amount: deposit_amount.to_string(),
                        }),
                        delegator_address: "cosmos2contract".to_string(),
                    }
                    .into(),
                },
                gas_limit: None,
                reply_on: ReplyOn::Success
            }
        );

        let expected_staked_amount = Decimal::to_uint_floor(
            Decimal::from_str(&deposit_amount.to_string())
                .unwrap()
                .checked_mul(Decimal::from_str(exchange_rate).unwrap())
                .unwrap(),
        );

        // ensure attributes are set
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "liquid_stake"),
                attr("native_amount", deposit_amount.to_string()),
                attr("lst_mint_amount", expected_staked_amount.to_string()),
                attr("exchange_rate", exchange_rate),
                attr("denom", NATIVE_DENOM),
                attr("receiver", "receiver"),
            ]
        );

        // ensure we can query the staked amount
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetStakedLiquidity {}).unwrap();
        let value: StakedLiquidityInfo = from_json(&res).unwrap();
        assert_eq!(deposit_amount, value.staked_amount_native);
    }

    #[test]
    fn liquid_stake_with_invalid_amount() {
        let (mut deps, _env, _info) = default_instantiate();

        let deposit_amount = Uint128::from(0u128);
        let exchange_rate = "0.825537496882794638";

        // Mock each pool in the querier
        deps.querier
            .mock_exchange_rate(MOCK_CHAIN_ID.to_string(), exchange_rate.to_string());

        // beneficiary can release it
        let info = mock_info("anyone", &coins(deposit_amount.u128(), "token"));
        let msg = ExecuteMsg::LiquidStake {
            receiver: Addr::unchecked("receiver"),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::PaymentError(e)) => {
                assert_eq!(e, "No funds sent")
            }
            _ => panic!("DO NOT ENTER HERE"),
        }
    }

    #[test]
    fn liquid_stake_with_invalid_denom() {
        let (mut deps, _env, _info) = default_instantiate();

        let deposit_amount = Uint128::from(1000u128);
        let exchange_rate = "0.825537496882794638";

        // Mock each pool in the querier
        deps.querier
            .mock_exchange_rate(MOCK_CHAIN_ID.to_string(), exchange_rate.to_string());

        // beneficiary can release it
        let info = mock_info("anyone", &coins(deposit_amount.u128(), "invalidtoken"));
        let msg = ExecuteMsg::LiquidStake {
            receiver: Addr::unchecked("receiver"),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::PaymentError(e)) => {
                assert_eq!(e, "Must send reserve token 'token'")
            }
            _ => panic!("DO NOT ENTER HERE"),
        }
    }

    #[test]
    fn handle_ls_reply_should_work() {
        let (mut deps, _env, _info) = default_instantiate();

        let msg = Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(Binary::from_base64("Cv0BeyJjb250cmFjdCI6Indvcm1ob2xlMXl3NHd2MnpxZzl4a242N3p2cTNhenllMHQ4aDB4OWtneWczZDUzanltMjRneHQ0OXZkeXM2czhoN2EiLCJkZW5vbSI6bnVsbCwicmVjaXBpZW50Ijoic2VpMWRrZHdkdmtueDBxYXY1Y3A1a3c2OG1rbjNyOTltM3N2a3lqZnZrenR3aDk3ZHYybG0wa3NqNnhyYWsiLCJhbW91bnQiOiIxMDAwIiwicmVsYXllciI6InNlaTF2aGttMnF2Nzg0cnVseDh5bHJ1MHpwdnl2dzNtM2N5OXgzeHlmdiIsImZlZSI6IjAifQ==").unwrap())
            })
        };

        let current_tx = LSInfo {
            receiver: Addr::unchecked("receiver"),
            prev_ls_token_balance: Uint128::new(1000u128),
        };
        CURRENT_TX.save(deps.as_mut().storage, &current_tx).unwrap();

        let res = handle_ls_reply(deps.as_mut(), mock_env(), msg).unwrap();

        assert_eq!(
            res.messages[0],
            SubMsg {
                id: 0,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "receiver".to_string(),
                    amount: vec![Coin {
                        denom: LIQUIDSTAKE_DENOM.to_string(),
                        amount: Uint128::new(1000u128),
                    }],
                }),
                gas_limit: None,
                reply_on: ReplyOn::Never
            }
        );
    }
}
