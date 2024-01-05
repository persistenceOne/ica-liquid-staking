use cosmwasm_std::{CheckedFromRatioError, OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not active")]
    NotActive {},

    #[error("Invalid denom : {e}")]
    InvalidDenom { e: PaymentError },

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Divide ratio error: {0}")]
    CheckedDivideRatioError(String),
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

impl From<CheckedFromRatioError> for ContractError {
    fn from(e: CheckedFromRatioError) -> Self {
        ContractError::CheckedDivideRatioError(e.to_string())
    }
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        match source {
            ContractError::Std(e) => e,
            e => StdError::generic_err(format!("{}", e)),
        }
    }
}
