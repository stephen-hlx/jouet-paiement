mod simple_transaction_processor;
use async_trait::async_trait;
#[cfg(test)]
pub use mock::{Blackhole, RecordSink};
pub use simple_transaction_processor::SimpleTransactionProcessor;

use crate::{
    account::account_transactor::AccountTransactionProcessorError,
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
            AccountTransactionProcessorError::CannotDisputeAgainstLockedAccount => todo!(),
            AccountTransactionProcessorError::NoTransactionFound => todo!(),
        }
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::{Transaction, TransactionProcessor, TransactionProcessorError};

    pub struct RecordSink {
        pub records: Arc<Mutex<Vec<Transaction>>>,
    }

    #[async_trait]
    impl TransactionProcessor for RecordSink {
        async fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError> {
            self.records.lock().unwrap().push(transaction);
            Ok(())
        }
    }

    pub struct Blackhole;
    #[async_trait]
    impl TransactionProcessor for Blackhole {
        async fn process(
            &self,
            _transaction: Transaction,
        ) -> Result<(), TransactionProcessorError> {
            Ok(())
        }
    }
}
