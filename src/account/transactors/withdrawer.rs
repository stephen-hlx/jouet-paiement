use crate::{
    account::{Account, AccountStatus, Withdrawal, WithdrawalStatus::Accepted},
    model::{Amount, TransactionId},
};

#[derive(Debug, Clone)]
pub(crate) enum WithdrawerError {
    AccountLocked,
    InsufficientFund,
}

pub(crate) trait Withdrawer {
    fn withdraw(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), WithdrawerError>;
}

pub(crate) struct SimpleWithdrawer;

impl Withdrawer for SimpleWithdrawer {
    fn withdraw(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<(), WithdrawerError> {
        if account.status == AccountStatus::Locked {
            return Err(WithdrawerError::AccountLocked);
        }
        if amount.0 != 0 && account.account_snapshot.available.0 < amount.0 {
            return Err(WithdrawerError::InsufficientFund);
        }
        account.account_snapshot.available.0 -= amount.0;
        account.withdrawals.insert(
            transaction_id,
            Withdrawal {
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

    use super::{Withdrawer, WithdrawerError};

    pub(crate) struct MockWithdrawer {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        return_vals: Arc<Mutex<Vec<Result<(), WithdrawerError>>>>,
    }

    impl MockWithdrawer {
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

        pub(crate) fn to_return(&self, result: Result<(), WithdrawerError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Withdrawer for MockWithdrawer {
        fn withdraw(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) -> Result<(), WithdrawerError> {
            self.actual_requests
                .lock()
                .unwrap()
                .push((account.clone(), transaction_id, amount));
            self.return_vals.lock().unwrap().remove(0)
        }
    }

    impl Drop for MockWithdrawer {
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
            transactors::withdrawer::WithdrawerError,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Withdrawal, WithdrawalStatus,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::SimpleWithdrawer;
    use super::Withdrawer;

    #[rstest]
    //    |-------------------- input -----------------------------| |--------------------- output ----------------------------------|
    //                                                   tx
    //     original_account,                             id, amount, expected_account
    //         avail, held,  existing withdrawals,                        avail, held, existing withdrawals
    #[case(active( 0,    0,  vec![]),                      2,      0, active( 0,    0, vec![(2, accepted_wdr(0))])                       )]
    #[case(active( 7,    0,  vec![]),                      2,      4, active( 3,    0, vec![(2, accepted_wdr(4))])                       )]
    #[case(active( 7,    0,  vec![]),                      2,      7, active( 0,    0, vec![(2, accepted_wdr(7))])                       )]
    #[case(active( 7,    0,  vec![(0, accepted_wdr(3))]),  2,      0, active( 7,    0, vec![(0, accepted_wdr(3)), (2, accepted_wdr(0))]) )]
    #[case(active( 7,    0,  vec![(0, held_wdr(3))]),      2,      0, active( 7,    0, vec![(0, held_wdr(3)), (2, accepted_wdr(0))])     )]
    #[case(active( 7,    0,  vec![(0, resolved_wdr(3))]),  2,      0, active( 7,    0, vec![(0, resolved_wdr(3)), (2, accepted_wdr(0))]) )]
    #[case(active( 7,    0,  vec![(0, chrgd_bck_wdr(3))]), 2,      0, active( 7,    0, vec![(0, chrgd_bck_wdr(3)), (2, accepted_wdr(0))]))]
    #[case(active( 7,    6,  vec![(0, accepted_wdr(3))]),  2,      4, active( 3,    6, vec![(0, accepted_wdr(3)), (2, accepted_wdr(4))]) )]
    #[case(active( 7,    6,  vec![(0, held_wdr(3))]),      2,      4, active( 3,    6, vec![(0, held_wdr(3)), (2, accepted_wdr(4))])     )]
    #[case(active( 7,    6,  vec![(0, resolved_wdr(3))]),  2,      4, active( 3,    6, vec![(0, resolved_wdr(3)), (2, accepted_wdr(4))]) )]
    #[case(active( 7,    6,  vec![(0, chrgd_bck_wdr(3))]), 2,      4, active( 3,    6, vec![(0, chrgd_bck_wdr(3)), (2, accepted_wdr(4))]))]
    #[case(active(-3,    6,  vec![(0, accepted_wdr(3))]),  2,      0, active(-3,    6, vec![(0, accepted_wdr(3)), (2, accepted_wdr(0))]) )]
    #[case(active(-3,    6,  vec![(0, held_wdr(3))]),      2,      0, active(-3,    6, vec![(0, held_wdr(3)), (2, accepted_wdr(0))])     )]
    #[case(active(-3,    6,  vec![(0, resolved_wdr(3))]),  2,      0, active(-3,    6, vec![(0, resolved_wdr(3)), (2, accepted_wdr(0))]) )]
    #[case(active(-3,    6,  vec![(0, chrgd_bck_wdr(3))]), 2,      0, active(-3,    6, vec![(0, chrgd_bck_wdr(3)), (2, accepted_wdr(0))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] amount_i64: i64,
        #[case] expected: Account,
    ) {
        let withdrawer = SimpleWithdrawer;
        withdrawer
            .withdraw(&mut original, transaction_id, amount(amount_i64))
            .unwrap();
        assert_eq!(original, expected);
    }

    #[rstest]
    //     original_available,   withdraw_amount
    #[rustfmt::skip(case)]
    #[case(5, 7)]
    #[case(0, 7)]
    #[case(                -1,                 7)]
    fn insufficient_fund_cases(#[case] original_available: i64, #[case] withdraw_amount: i64) {
        let mut account = active(original_available, 0, vec![]);
        let withdrawer = SimpleWithdrawer;
        assert_matches!(
            withdrawer.withdraw(&mut account, 0, amount(withdraw_amount)),
            Err(WithdrawerError::InsufficientFund)
        );
    }

    #[test]
    fn withdrawal_from_locked_account_returns_error() {
        let mut account = account(Locked, 0, 0, vec![]);
        let withdrawer = SimpleWithdrawer;
        assert_matches!(
            withdrawer.withdraw(&mut account, 1, amount(10)),
            Err(WithdrawerError::AccountLocked)
        );
    }

    fn active(available: i64, held: i64, withdrawals: Vec<(TransactionId, Withdrawal)>) -> Account {
        account(Active, available, held, withdrawals)
    }

    fn account(
        status: AccountStatus,
        available: i64,
        held: i64,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        Account {
            client_id: 1234,
            status,
            account_snapshot: AccountSnapshot::new(available, held),
            deposits: HashMap::new(),
            withdrawals: withdrawals.into_iter().collect(),
        }
    }

    fn accepted_wdr(amount_i64: i64) -> Withdrawal {
        withdrawal(amount_i64, WithdrawalStatus::Accepted)
    }

    fn held_wdr(amount_i64: i64) -> Withdrawal {
        withdrawal(amount_i64, WithdrawalStatus::Held)
    }

    fn resolved_wdr(amount_i64: i64) -> Withdrawal {
        withdrawal(amount_i64, WithdrawalStatus::Resolved)
    }

    fn chrgd_bck_wdr(amount_i64: i64) -> Withdrawal {
        withdrawal(amount_i64, WithdrawalStatus::ChargedBack)
    }

    fn withdrawal(amount_i64: i64, status: WithdrawalStatus) -> Withdrawal {
        Withdrawal {
            amount: amount(amount_i64),
            status,
        }
    }

    fn amount(amount: i64) -> Amount {
        Amount4DecimalBased(amount)
    }
}
