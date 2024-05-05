mod credit_backcharger;
mod credit_debit_backcharger;
use crate::{account::Account, model::TransactionId};
pub(crate) use credit_debit_backcharger::CreditDebitBackcharger;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BackchargerError {
    AccountLocked,
    CannotChargebackNonDisputedTransaction(TransactionId),
    NoTransactionFound,
}

pub(crate) trait Backcharger {
    fn chargeback(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<(), BackchargerError>;
}

#[cfg(test)]
pub(crate) mod mock {

    use std::sync::{Arc, Mutex};

    use crate::{account::Account, model::TransactionId};

    use super::{Backcharger, BackchargerError};

    pub(crate) struct MockBackcharger {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        return_vals: Arc<Mutex<Vec<Result<(), BackchargerError>>>>,
    }

    impl MockBackcharger {
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

        pub(crate) fn to_return(&self, result: Result<(), BackchargerError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Backcharger for MockBackcharger {
        fn chargeback(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
        ) -> Result<(), BackchargerError> {
            self.actual_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id));
            self.return_vals.lock().unwrap().remove(0)
        }
    }

    impl Drop for MockBackcharger {
        fn drop(&mut self) {
            assert_eq!(
                *self.actual_requests.lock().unwrap(),
                *self.expected_requests.lock().unwrap()
            );
            assert!(self.return_vals.lock().unwrap().is_empty());
        }
    }
}
