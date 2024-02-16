use cosmwasm_std::{OverflowError, StdError};
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

    #[error("No funds")]
    NoFunds {},

    #[error("Too many funds")]
    TooManyFunds {},

    #[error("Invalid denom: {denom}")]
    InvalidDenom { denom: String },

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Payment error: {0}")]
    PaymentError(String),

    #[error("Invalid recovery address")]
    InvalidRecoveryAddress {},

    #[error("Unknown reply id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Parse reply error: {0}")]
    ParseReplyError(String),

    #[error("LS failed to return data in its response")]
    LSResponseDataMissing,

    #[error("Subcall error: {0}")]
    SubcallError(String),

    #[error("No claimable tokens")]
    NoClaimableTokens {},
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

impl From<PaymentError> for ContractError {
    fn from(e: PaymentError) -> Self {
        ContractError::PaymentError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::OverflowOperation;

    use super::*;

    #[test]
    fn test_std_error() {
        let err = OverflowError::new(OverflowOperation::Sub, 123u128, 456u128);
        let contract_err: ContractError = err.into();
        assert_eq!(
            contract_err,
            StdError::overflow(OverflowError::new(OverflowOperation::Sub, 123u128, 456u128)).into()
        );
    }

    #[test]
    fn test_payment_error() {
        let err = PaymentError::MissingDenom("denom".to_string());
        let contract_err: ContractError = err.into();
        assert_eq!(
            contract_err,
            ContractError::PaymentError("Must send reserve token 'denom'".to_string())
        );
    }

    #[test]
    fn test_parse_reply_error() {
        let err = ContractError::ParseReplyError("parse error".to_string());
        let contract_err: ContractError = err.into();
        assert_eq!(
            contract_err,
            ContractError::ParseReplyError("parse error".to_string())
        );
    }
}
