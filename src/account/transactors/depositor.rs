use crate::{
    account::{
        account_transactor::SuccessStatus, Account, AccountStatus, Deposit, DepositStatus::Accepted,
    },
    model::{Amount, TransactionId},
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DepositorError {
    AccountLocked,
}

pub(crate) trait Depositor {
    fn deposit(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<SuccessStatus, DepositorError>;
}

pub(crate) struct SimpleDepositor;

impl Depositor for SimpleDepositor {
    fn deposit(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<SuccessStatus, DepositorError> {
        if account.status == AccountStatus::Locked {
            return Err(DepositorError::AccountLocked);
        }
        match account.deposits.get(&transaction_id) {
            Some(existing) => {
                assert_eq!(existing.amount, amount);
                Ok(SuccessStatus::Duplicate)
            }
            None => {
                account.account_snapshot.available.0 += amount.0;
                account.deposits.insert(
                    transaction_id,
                    Deposit {
                        amount,
                        status: Accepted,
                    },
                );
                Ok(SuccessStatus::Transacted)
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod mock {

    use std::sync::{Arc, Mutex};

    use crate::{
        account::{account_transactor::SuccessStatus, Account},
        model::{Amount, TransactionId},
    };

    use super::{Depositor, DepositorError};

    pub(crate) struct MockDepositor {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        return_vals: Arc<Mutex<Vec<Result<SuccessStatus, DepositorError>>>>,
    }

    impl MockDepositor {
        pub(crate) fn new() -> Self {
            Self {
                expected_requests: Arc::new(Mutex::new(Vec::new())),
                actual_requests: Arc::new(Mutex::new(Vec::new())),
                return_vals: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub(crate) fn expect(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) {
            self.expected_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id, amount));
        }

        pub(crate) fn to_return(&self, result: Result<SuccessStatus, DepositorError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Depositor for MockDepositor {
        fn deposit(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) -> Result<SuccessStatus, DepositorError> {
            self.actual_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id, amount));
            self.return_vals.lock().unwrap().remove(0)
        }
    }

    impl Drop for MockDepositor {
        fn drop(&mut self) {
            assert_eq!(
                *self.actual_requests.lock().unwrap(),
                *self.expected_requests.lock().unwrap()
            );
            assert!(self.return_vals.lock().unwrap().is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assert_matches::assert_matches;

    use rstest::rstest;

    use crate::{
        account::{
            account_transactor::SuccessStatus,
            account_transactor::SuccessStatus::Duplicate,
            account_transactor::SuccessStatus::Transacted,
            transactors::depositor::DepositorError,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::Depositor;
    use super::SimpleDepositor;

    #[rstest]
    //    |------------------- input ------------------| |-------------------------------------- output -------------------------------------------------------|
    //
    //     original_account,                   tx_id,                                                expected_account
    //        avail, deposits,                   amount, expected_status                             avail,  deposits
    #[case(active(0, vec![]),                      0, 3, Ok(Transacted),                             active(3, vec![(0, accepted_dep(3))])                      )]
    #[case(active(3, vec![(0, accepted_dep(3))]),  0, 3, Ok(Duplicate),                              active(3, vec![(0, accepted_dep(3))])                      )]
    #[case(active(3, vec![(0, held_dep(3))]),      0, 3, Ok(Duplicate),                              active(3, vec![(0, held_dep(3))])                          )]
    #[case(active(3, vec![(0, resolved_dep(3))]),  0, 3, Ok(Duplicate),                              active(3, vec![(0, resolved_dep(3))])                      )]
    #[case(active(3, vec![(0, chrgd_bck_dep(3))]), 0, 3, Ok(Duplicate),                              active(3, vec![(0, chrgd_bck_dep(3))])                     )]
    #[case(active(3, vec![(0, accepted_dep(3))]),  2, 5, Ok(Transacted),                             active(8, vec![(0, accepted_dep(3)), (2, accepted_dep(5))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] amount_i64: i64,
        #[case] expected_status: Result<SuccessStatus, DepositorError>,
        #[case] expected: Account,
    ) {
        let depositor = SimpleDepositor;
        assert_eq!(
            depositor.deposit(&mut original, transaction_id, amount(amount_i64)),
            expected_status
        );
        assert_eq!(original, expected);
    }

    #[test]
    fn deposit_to_locked_account_returns_error() {
        let mut account = account(Locked, 0, 0, vec![]);
        let depositor = SimpleDepositor;
        assert_matches!(
            depositor.deposit(&mut account, 1, amount(10)),
            Err(DepositorError::AccountLocked)
        );
    }

    fn active(available: i64, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Active, available, 0, deposits)
    }

    fn account(
        status: AccountStatus,
        available: i64,
        held: i64,
        deposits: Vec<(TransactionId, Deposit)>,
    ) -> Account {
        Account {
            client_id: 1234,
            status,
            account_snapshot: AccountSnapshot::new(available, held),
            deposits: deposits.into_iter().collect(),
            withdrawals: HashMap::new(),
        }
    }

    fn accepted_dep(amount_i64: i64) -> Deposit {
        deposit(amount_i64, DepositStatus::Accepted)
    }

    fn held_dep(amount_i64: i64) -> Deposit {
        deposit(amount_i64, DepositStatus::Held)
    }

    fn resolved_dep(amount_i64: i64) -> Deposit {
        deposit(amount_i64, DepositStatus::Resolved)
    }

    fn chrgd_bck_dep(amount_i64: i64) -> Deposit {
        deposit(amount_i64, DepositStatus::ChargedBack)
    }

    fn deposit(amount_i64: i64, status: DepositStatus) -> Deposit {
        Deposit {
            amount: amount(amount_i64),
            status,
        }
    }

    fn amount(amount: i64) -> Amount {
        Amount4DecimalBased(amount)
    }
}
