use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;

use super::{TransactionProcessor, TransactionProcessorError};
use crate::account::account_transactor::AccountTransactor;
use crate::model::Transaction;
use crate::{account::Account, model::ClientId};

pub struct SimpleTransactionProcessor {
    accounts: Arc<DashMap<ClientId, Account>>,
    account_transaction_processor: Box<dyn AccountTransactor + 'static + Send + Sync>,
}

#[async_trait]
impl TransactionProcessor for SimpleTransactionProcessor {
    async fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError> {
        let client_id = transaction.client_id;
        let mut binding = self
            .accounts
            .entry(client_id)
            .or_insert_with(|| Account::active(client_id));
        let account = binding.value_mut();

        self.account_transaction_processor
            .transact(account, transaction)?;
        Ok(())
    }
}

impl SimpleTransactionProcessor {
    pub fn new(
        accounts: Arc<DashMap<ClientId, Account>>,
        account_transaction_processor: Box<dyn AccountTransactor + 'static + Send + Sync>,
    ) -> Self {
        Self {
            accounts,
            account_transaction_processor,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashmap::DashMap;

    use ordered_float::OrderedFloat;

    use crate::{
        account::{
            account_transactor::{AccountTransactor, AccountTransactorError},
            Account,
        },
        model::{Amount, ClientId, Transaction, TransactionId, TransactionKind},
        transaction_processor::TransactionProcessor,
    };

    use super::SimpleTransactionProcessor;

    const CLIENT_ID: ClientId = 123;
    const TRANSACTION_ID: TransactionId = 456;
    const AMOUNT: Amount = OrderedFloat(7.89);

    pub struct MockAccountTransactionProcessor {
        expected_request: (Account, Transaction),
        return_val: Result<(), AccountTransactorError>,
    }

    impl AccountTransactor for MockAccountTransactionProcessor {
        fn transact(
            &self,
            account: &mut Account,
            transaction: Transaction,
        ) -> Result<(), AccountTransactorError> {
            let (expected_account, expected_transaction) = self.expected_request.clone();
            assert_eq!(*account, expected_account);
            assert_eq!(transaction, expected_transaction);
            self.return_val.clone()
        }
    }

    #[tokio::test]
    async fn loads_account_and_processes_the_transaction() {
        let transaction = Transaction {
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            kind: TransactionKind::Deposit { amount: AMOUNT },
        };
        let account = Account::active(CLIENT_ID);
        let accounts = Arc::new(DashMap::new());
        accounts.insert(CLIENT_ID, account.clone());
        let account_transaction_processor = MockAccountTransactionProcessor {
            expected_request: (account.clone(), transaction.clone()),
            return_val: Ok(()),
        };
        let transaction_processor =
            SimpleTransactionProcessor::new(accounts, Box::new(account_transaction_processor));
        transaction_processor.process(transaction).await.unwrap();
    }

    #[tokio::test]
    async fn creates_account_if_it_does_not_already_exist_and_processes_the_transaction() {
        let transaction = Transaction {
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            kind: TransactionKind::Deposit { amount: AMOUNT },
        };
        let account = Account::active(CLIENT_ID);
        let accounts = Arc::new(DashMap::new());
        let account_transaction_processor = MockAccountTransactionProcessor {
            expected_request: (account.clone(), transaction.clone()),
            return_val: Ok(()),
        };
        let transaction_processor = SimpleTransactionProcessor::new(
            accounts.clone(),
            Box::new(account_transaction_processor),
        );
        transaction_processor.process(transaction).await.unwrap();
        assert_eq!(
            *accounts.get(&CLIENT_ID).unwrap().value(),
            Account::active(CLIENT_ID)
        );
    }
}
