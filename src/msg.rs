use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    /// LS token prefix used to identify LS tokens
    /// e.g. "stk/"
    pub ls_prefix: String,
}

#[cw_serde]
pub struct LsConfig {
    /// admin address
    pub admin: Addr,
    /// Flag to enable/disable the contract
    pub active: bool,
    /// LS token prefix
    pub ls_prefix: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Liquid stake tokens
    LiquidStake {
        /// Receiver of the liquid staked tokens on Persistence chain
        receiver: Addr,
    },
    /// Update the contract configuration
    UpdateConfig {
        /// Flag to enable/disable the contract
        active: Option<bool>,
        /// LS token prefix
        ls_prefix: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LsConfig)]
    LsConfig {},
}
