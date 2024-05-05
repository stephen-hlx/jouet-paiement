mod credit_debit_disputer;
mod credit_disputer;
pub(crate) use credit_debit_disputer::CreditDebitDisputer;

use crate::{account::Account, model::TransactionId};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DisputerError {
    AccountLocked,
    NoTransactionFound,
}

pub(crate) trait Disputer {
    fn dispute(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<(), DisputerError>;
}

#[cfg(test)]
pub(crate) mod mock {

    use std::sync::{Arc, Mutex};

    use crate::{account::Account, model::TransactionId};

    use super::{Disputer, DisputerError};

    pub(crate) struct MockDisputer {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        return_vals: Arc<Mutex<Vec<Result<(), DisputerError>>>>,
    }

    impl MockDisputer {
        pub(crate) fn new() -> Self {
            Self {
                expected_requests: Arc::new(Mutex::new(Vec::new())),
                actual_requests: Arc::new(Mutex::new(Vec::new())),
                return_vals: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub(crate) fn expect(&self, account: &mut Account, transaction_id: TransactionId) {
            self.expected_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id));
        }

        pub(crate) fn to_return(&self, result: Result<(), DisputerError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Disputer for MockDisputer {
        fn dispute(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
        ) -> Result<(), DisputerError> {
            self.actual_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id));
            self.return_vals.lock().unwrap().remove(0)
        }
    }

    impl Drop for MockDisputer {
        fn drop(&mut self) {
            assert_eq!(
                *self.actual_requests.lock().unwrap(),
                *self.expected_requests.lock().unwrap()
            );
            assert!(self.return_vals.lock().unwrap().is_empty());
        }
    }
}
