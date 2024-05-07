// mod credit_debit_resolver;
mod credit_resolver;
use crate::{
    account::{account_transactor::SuccessStatus, Account},
    model::TransactionId,
};
pub(crate) use credit_resolver::CreditResolver;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResolverError {
    AccountLocked,
    NonDisputedTransaction,
    NoTransactionFound,
}

pub(crate) trait Resolver {
    fn resolve(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<SuccessStatus, ResolverError>;
}

#[cfg(test)]
pub(crate) mod mock {

    use std::sync::{Arc, Mutex};

    use crate::{
        account::{account_transactor::SuccessStatus, Account},
        model::TransactionId,
    };

    use super::{Resolver, ResolverError};

    pub(crate) struct MockResolver {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId)>>>,
        return_vals: Arc<Mutex<Vec<Result<SuccessStatus, ResolverError>>>>,
    }

    impl MockResolver {
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

        pub(crate) fn to_return(&self, result: Result<SuccessStatus, ResolverError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Resolver for MockResolver {
        fn resolve(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
        ) -> Result<SuccessStatus, ResolverError> {
            self.actual_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id));
            self.return_vals.lock().unwrap().remove(0)
        }
    }

    impl Drop for MockResolver {
        fn drop(&mut self) {
            assert_eq!(
                *self.actual_requests.lock().unwrap(),
                *self.expected_requests.lock().unwrap()
            );
            assert!(self.return_vals.lock().unwrap().is_empty());
        }
    }
}
