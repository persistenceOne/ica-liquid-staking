use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not active")]
    NotActive {},

    #[error("Invalid denom {denom}, expected {expected}")]
    InvalidDenom { denom: String, expected: String },

    #[error("Invalid amount")]
    InvalidAmount {},
}
