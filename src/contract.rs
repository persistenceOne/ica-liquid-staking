#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    execute::{try_liquid_staking, LS_REPLY_ID},
    msg::{ExecuteMsg, InstantiateMsg, LsConfig, QueryMsg},
    query,
    reply::handle_ls_reply,
    state::LS_CONFIG,
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

    let ls_config = LsConfig {
        active: true,
        ls_prefix: msg.ls_prefix.clone(),
    };
    LS_CONFIG.save(deps.storage, &ls_config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("active", "true")
        .add_attribute("ls_prefix", msg.ls_prefix))
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
        QueryMsg::LsConfig {} => to_json_binary(&query::query_ls_config(deps)?),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::execute::DENOM_TRACE_QUERY_TYPE;
    use crate::state::{LSInfo, CURRENT_TX};

    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
    use cosmwasm_std::{
        attr, coins, from_json, Addr, BalanceResponse, BankMsg, BankQuery, Coin, ContractResult,
        CosmosMsg, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, ReplyOn, SubMsg,
        SubMsgResponse, SystemError, SystemResult, Uint128,
    };
    use osmosis_std::types::ibc::applications::transfer::v1::{
        DenomTrace, QueryDenomTraceRequest, QueryDenomTraceResponse,
    };
    use persistence_std::types::cosmos::base::v1beta1::Coin as StdCoin;
    use persistence_std::types::pstake::liquidstakeibc::v1beta1::MsgLiquidStake;

    use prost::Message;

    const NATIVE_IBC_DENOM: &str =
        "ibc/C8A74ABBE2AF892E15680D916A7C22130585CE5704F9B17A10F184A90D53BECA";
    const NATIVE_BASE_DENOM: &str = "uatom";
    const LIQUIDSTAKE_DENOM: &str = "stk/uatom";

    pub struct WasmMockQuerier {
        pub denom_trace: HashMap<String, QueryDenomTraceResponse>,
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
                denom_trace: HashMap::new(),
            }
        }

        fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match request {
                QueryRequest::Stargate { path, data } => {
                    if path == DENOM_TRACE_QUERY_TYPE {
                        let query_denom_trace_request =
                            QueryDenomTraceRequest::decode(data.as_slice()).unwrap();
                        match self.denom_trace.get(&query_denom_trace_request.hash) {
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

        pub fn mock_denom_trace(&mut self, ibc_hash: String) {
            self.denom_trace.insert(
                ibc_hash,
                QueryDenomTraceResponse {
                    denom_trace: Some(DenomTrace {
                        path: "transfer/channel-0".to_string(),
                        base_denom: NATIVE_BASE_DENOM.to_string(),
                    }),
                },
            );
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
            ls_prefix: "stk/".to_string(),
        };

        let resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            resp.attributes,
            vec![
                attr("method", "instantiate"),
                attr("owner", "creator"),
                attr("active", "true"),
                attr("ls_prefix", "stk/"),
            ]
        );

        // Mock each pool in the querier
        deps.querier.mock_denom_trace(NATIVE_IBC_DENOM.to_string());

        (deps, env, info)
    }

    #[test]
    fn proper_initialization() {
        let (deps, _env, _info) = default_instantiate();

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LsConfig {}).unwrap();
        let value: LsConfig = from_json(&res).unwrap();
        assert_eq!(true, value.active);
    }

    #[test]
    fn liquid_stake() {
        let (mut deps, _env, _info) = default_instantiate();

        let deposit_amount = Uint128::from(2000u128);

        // beneficiary can release it
        let info = mock_info("anyone", &coins(deposit_amount.into(), NATIVE_IBC_DENOM));
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
                            denom: NATIVE_IBC_DENOM.to_string(),
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

        // ensure attributes are set
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "liquid_stake"),
                attr("native_amount", deposit_amount.to_string()),
                attr("native_ibc_denom", NATIVE_IBC_DENOM),
                attr("native_base_denom", NATIVE_BASE_DENOM),
                attr("ls_token_denom", LIQUIDSTAKE_DENOM),
                attr("receiver", "receiver"),
            ]
        );
    }

    #[test]
    fn handle_ls_reply_should_work() {
        let (mut deps, _env, _info) = default_instantiate();

        let msg = Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(Binary::from_base64("AXsiY29udHJhY3QiOiJjb3Ntb3MyY29udHJhY3QiLCJkZW5vbSI6bnVsbCwicmVjaXBpZW50IjoicmVjZWl2ZXIiLCJhbW91bnQiOiIxMDAwIiwicmVsYXllciI6InJlbGF5ZXIiLCJmZWUiOiIwIn0=").unwrap())
            })
        };

        let current_tx = LSInfo {
            receiver: Addr::unchecked("receiver"),
            ibc_denom: NATIVE_IBC_DENOM.to_string(),
            ls_token_denom: LIQUIDSTAKE_DENOM.to_string(),
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
