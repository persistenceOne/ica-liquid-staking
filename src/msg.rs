use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    /// LS token prefix
    pub ls_prefix: String,
}

#[cw_serde]
pub struct LsConfig {
    /// Flag to enable/disable the contract
    pub active: bool,
    /// LS token prefix
    pub ls_prefix: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    LiquidStake { receiver: Addr },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LsConfig)]
    LsConfig {},
}
