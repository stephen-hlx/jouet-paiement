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
        account.account_snapshot.available += amount;
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
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        account::Account,
        model::{Amount, TransactionId},
    };

    use super::{Depositor, DepositorError};

    pub(crate) struct MockDepositor {
        expected_requests: Rc<RefCell<Vec<(Account, TransactionId, Amount)>>>,
        actual_requests: Rc<RefCell<Vec<(Account, TransactionId, Amount)>>>,
        return_vals: Rc<RefCell<Vec<Result<(), DepositorError>>>>,
    }

    impl MockDepositor {
        pub(crate) fn new() -> Self {
            Self {
                expected_requests: Rc::new(RefCell::new(Vec::new())),
                actual_requests: Rc::new(RefCell::new(Vec::new())),
                return_vals: Rc::new(RefCell::new(Vec::new())),
            }
        }

        pub(crate) fn expect(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) {
            self.expected_requests
                .borrow_mut()
                .push((account.clone(), transaction_id, amount));
        }

        pub(crate) fn to_return(&self, result: Result<(), DepositorError>) {
            self.return_vals.borrow_mut().push(result);
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
                .borrow_mut()
                .push((account.clone(), transaction_id, amount));
            self.return_vals.borrow_mut().remove(0)
        }
    }

    impl Drop for MockDepositor {
        fn drop(&mut self) {
            assert_eq!(*self.actual_requests, *self.expected_requests);
            assert!(self.return_vals.borrow().is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assert_matches::assert_matches;
    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        account::{
            processor::depositor::DepositorError,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus,
        },
        model::{Amount, TransactionId},
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
        #[case] amount_u32: u32,
        #[case] expected: Account,
    ) {
        let depositor = SimpleDepositor;
        depositor
            .deposit(&mut original, transaction_id, amount(amount_u32))
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

    fn active(available: i32, held: u32, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Active, available, held, deposits)
    }

    fn account(
        status: AccountStatus,
        available: i32,
        held: u32,
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

    fn accepted_dep(amount_u32: u32) -> Deposit {
        deposit(amount_u32, DepositStatus::Accepted)
    }

    fn held_dep(amount_u32: u32) -> Deposit {
        deposit(amount_u32, DepositStatus::Held)
    }

    fn resolved_dep(amount_u32: u32) -> Deposit {
        deposit(amount_u32, DepositStatus::Resolved)
    }

    fn chrgd_bck_dep(amount_u32: u32) -> Deposit {
        deposit(amount_u32, DepositStatus::ChargedBack)
    }

    fn deposit(amount_u32: u32, status: DepositStatus) -> Deposit {
        Deposit {
            amount: amount(amount_u32),
            status,
        }
    }

    fn amount(amount: u32) -> Amount {
        OrderedFloat(amount as f32)
    }
}
