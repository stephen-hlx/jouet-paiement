mod simple_transaction_processor;
use async_trait::async_trait;
#[cfg(test)]
pub use mock::{Blackhole, RecordSink};
pub use simple_transaction_processor::SimpleTransactionProcessor;
use thiserror::Error;

use crate::{account::account_transactor::AccountTransactorError, model::Transaction};

/// The transction processor.
/// It takes in a transaction and processes it based on previously seen
/// transactions. The transaction may be rejected if there is an error occurred
/// during the process of it.
#[async_trait]
pub trait TransactionProcessor {
    async fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError>;
}

#[derive(Debug, Error)]
pub enum TransactionProcessorError {
    #[error("Failed to process transaction: {0:?}. Error: {1}")]
    AccountTransactionError(Transaction, AccountTransactorError),
}

#[cfg(test)]
pub(crate) mod mock {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::model::Transaction;

    use super::{TransactionProcessor, TransactionProcessorError};

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
