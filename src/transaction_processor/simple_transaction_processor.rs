use std::sync::Arc;

use dashmap::DashMap;

use super::{Transaction, TransactionProcessor, TransactionProcessorError};
use crate::account::account_transaction_processor::AccountTransactionProcessorTrait;
use crate::{account::Account, model::ClientId};

struct SimpleTransactionProcessor {
    accounts: Arc<DashMap<ClientId, Account>>,
    account_transaction_processor: Box<dyn AccountTransactionProcessorTrait>,
}

impl TransactionProcessor for SimpleTransactionProcessor {
    fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError> {
        let client_id = transaction.client_id;
        let mut binding = self
            .accounts
            .entry(client_id)
            .or_insert_with(|| Account::active(client_id));
        let account = binding.value_mut();

        self.account_transaction_processor
            .process(account, transaction)?;
        Ok(())
    }
}

impl SimpleTransactionProcessor {
    fn new(account_transaction_processor: Box<dyn AccountTransactionProcessorTrait>) -> Self {
        Self {
            accounts: Arc::new(DashMap::new()),
            account_transaction_processor,
        }
    }

    #[cfg(test)]
    fn new_for_test(
        accounts: Arc<DashMap<ClientId, Account>>,
        account_transaction_processor: Box<dyn AccountTransactionProcessorTrait>,
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
            account_transaction_processor::{
                AccountTransactionProcessorError, AccountTransactionProcessorTrait,
            },
            Account,
        },
        model::{Amount, ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionProcessor},
    };

    use super::SimpleTransactionProcessor;

    const CLIENT_ID: ClientId = 123;
    const TRANSACTION_ID: TransactionId = 456;
    const AMOUNT: Amount = OrderedFloat(7.89);

    pub struct MockAccountTransactionProcessor {
        expected_request: (Account, Transaction),
        return_val: Result<(), AccountTransactionProcessorError>,
    }

    impl AccountTransactionProcessorTrait for MockAccountTransactionProcessor {
        fn process(
            &self,
            account: &mut Account,
            transaction: Transaction,
        ) -> Result<(), AccountTransactionProcessorError> {
            let (expected_account, expected_transaction) = self.expected_request.clone();
            assert_eq!(*account, expected_account);
            assert_eq!(transaction, expected_transaction);
            self.return_val.clone()
        }
    }

    #[test]
    fn loads_account_and_processes_the_transaction() {
        let transaction = Transaction {
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            kind: crate::transaction_processor::TransactionKind::Deposit { amount: AMOUNT },
        };
        let account = Account::active(CLIENT_ID);
        let accounts = Arc::new(DashMap::new());
        accounts.insert(CLIENT_ID, account.clone());
        let account_transaction_processor = MockAccountTransactionProcessor {
            expected_request: (account.clone(), transaction.clone()),
            return_val: Ok(()),
        };
        let transaction_processor = SimpleTransactionProcessor::new_for_test(
            accounts,
            Box::new(account_transaction_processor),
        );
        transaction_processor.process(transaction).unwrap();
    }

    #[test]
    fn creates_account_if_it_does_not_already_exist_and_processes_the_transaction() {
        let transaction = Transaction {
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            kind: crate::transaction_processor::TransactionKind::Deposit { amount: AMOUNT },
        };
        let account = Account::active(CLIENT_ID);
        let accounts = Arc::new(DashMap::new());
        let account_transaction_processor = MockAccountTransactionProcessor {
            expected_request: (account.clone(), transaction.clone()),
            return_val: Ok(()),
        };
        let transaction_processor = SimpleTransactionProcessor::new_for_test(
            accounts.clone(),
            Box::new(account_transaction_processor),
        );
        transaction_processor.process(transaction).unwrap();
        assert_eq!(
            *accounts.get(&CLIENT_ID).unwrap().value(),
            Account::active(CLIENT_ID)
        );
    }
}
