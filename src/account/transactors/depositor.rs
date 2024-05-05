use crate::{
    account::{Account, AccountStatus, Deposit, DepositStatus::Accepted},
    model::{Amount, TransactionId},
};

#[derive(Debug, Clone)]
pub(crate) enum DepositorError {
    AccountLocked,
}

pub(crate) trait Depositor {
    fn deposit(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), DepositorError>;
}

pub(crate) struct SimpleDepositor;

impl Depositor for SimpleDepositor {
    fn deposit(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), DepositorError> {
        if account.status == AccountStatus::Locked {
            return Err(DepositorError::AccountLocked);
        }
        account.account_snapshot.available.0 += amount.0;
        account.deposits.insert(
            transaction_id,
            Deposit {
                amount,
                status: Accepted,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod mock {

    use std::sync::{Arc, Mutex};

    use crate::{
        account::Account,
        model::{Amount, TransactionId},
    };

    use super::{Depositor, DepositorError};

    pub(crate) struct MockDepositor {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        return_vals: Arc<Mutex<Vec<Result<(), DepositorError>>>>,
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

        pub(crate) fn to_return(&self, result: Result<(), DepositorError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Depositor for MockDepositor {
        fn deposit(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) -> Result<(), DepositorError> {
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
    //    |-------------------- input -----------------------------| |--------------------- output ----------------------------------|
    //                                                   tx
    //     original_account,                             id, amount, expected_account
    //         avail, held, deposits,                                   avail,  held, deposits
    #[case(active(-3,    0, vec![]),                      2,     10, active( 7,    0, vec![(2, accepted_dep(10))])                       )]
    #[case(active( 0,    0, vec![]),                      2,     10, active(10,    0, vec![(2, accepted_dep(10))])                       )]
    #[case(active( 3,    0, vec![]),                      2,     10, active(13,    0, vec![(2, accepted_dep(10))])                       )]
    #[case(active( 0,    0, vec![(0, accepted_dep(3))]),  2,     10, active(10,    0, vec![(0, accepted_dep(3)), (2, accepted_dep(10))]) )]
    #[case(active( 0,    0, vec![(0, held_dep(3))]),      2,     10, active(10,    0, vec![(0, held_dep(3)), (2, accepted_dep(10))])     )]
    #[case(active( 0,    0, vec![(0, resolved_dep(3))]),  2,     10, active(10,    0, vec![(0, resolved_dep(3)), (2, accepted_dep(10))]) )]
    #[case(active( 0,    0, vec![(0, chrgd_bck_dep(3))]), 2,     10, active(10,    0, vec![(0, chrgd_bck_dep(3)), (2, accepted_dep(10))]))]
    #[case(active( 2,    6, vec![(0, accepted_dep(3))]),  2,     10, active(12,    6, vec![(0, accepted_dep(3)), (2, accepted_dep(10))]) )]
    #[case(active( 2,    6, vec![(0, held_dep(3))]),      2,     10, active(12,    6, vec![(0, held_dep(3)), (2, accepted_dep(10))])     )]
    #[case(active( 2,    6, vec![(0, resolved_dep(3))]),  2,     10, active(12,    6, vec![(0, resolved_dep(3)), (2, accepted_dep(10))]) )]
    #[case(active( 2,    6, vec![(0, chrgd_bck_dep(3))]), 2,     10, active(12,    6, vec![(0, chrgd_bck_dep(3)), (2, accepted_dep(10))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] amount_i64: i64,
        #[case] expected: Account,
    ) {
        let depositor = SimpleDepositor;
        depositor
            .deposit(&mut original, transaction_id, amount(amount_i64))
            .unwrap();
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

    fn active(available: i64, held: i64, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Active, available, held, deposits)
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
