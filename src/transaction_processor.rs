mod simple_transaction_processor;
use async_trait::async_trait;
pub use simple_transaction_processor::SimpleTransactionProcessor;

use crate::{
    account::account_transaction_processor::AccountTransactionProcessorError,
    model::{Amount, ClientId, TransactionId},
};

/// The transaction structure accepted by this application.
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub kind: TransactionKind,
}

/// The kinds of transactions.
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionKind {
    Deposit { amount: Amount },
    Withdrawal { amount: Amount },
    Dispute,
    Resolve,
    ChargeBack,
}

/// The transction processor.
/// It takes in a transaction and processes it based on previously seen
/// transactions. The transaction may be rejected if there is an error occurred
/// during the process of it.
#[async_trait]
pub trait TransactionProcessor {
    async fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError>;
}

#[derive(Debug)]
pub enum TransactionProcessorError {
    // todo: need an ID
    AccountLocked,
    InvalidTransaction(Transaction),

    InternalError(String),
}

impl From<AccountTransactionProcessorError> for TransactionProcessorError {
    fn from(err: AccountTransactionProcessorError) -> Self {
        match err {
            AccountTransactionProcessorError::MismatchTransactionKind => todo!(),
            AccountTransactionProcessorError::CannotDepositToLockedAccount => Self::AccountLocked,
            AccountTransactionProcessorError::CannotWithdrawFromLockedAccount => todo!(),
            AccountTransactionProcessorError::InsufficientFundForWithdrawal => todo!(),
        }
    }
}
