use crate::{
    account::{
        account_transactor::SuccessStatus, Account, AccountStatus, Withdrawal,
        WithdrawalStatus::Accepted,
    },
    model::{Amount, TransactionId},
};

#[derive(Debug, Clone, PartialEq)]
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
    ) -> Result<SuccessStatus, WithdrawerError>;
}

pub(crate) struct SimpleWithdrawer;

impl Withdrawer for SimpleWithdrawer {
    fn withdraw(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
        amount: Amount,
    ) -> Result<SuccessStatus, WithdrawerError> {
        if account.status == AccountStatus::Locked {
            return Err(WithdrawerError::AccountLocked);
        }
        if amount.0 != 0 && account.account_snapshot.available.0 < amount.0 {
            return Err(WithdrawerError::InsufficientFund);
        }
        match account.withdrawals.get(&transaction_id) {
            Some(existing) => {
                assert_eq!(existing.amount, amount);
                Ok(SuccessStatus::Duplicate)
            }
            None => {
                account.account_snapshot.available.0 -= amount.0;
                account.withdrawals.insert(
                    transaction_id,
                    Withdrawal {
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

    use super::{Withdrawer, WithdrawerError};

    pub(crate) struct MockWithdrawer {
        expected_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        actual_requests: Arc<Mutex<Vec<(Account, TransactionId, Amount)>>>,
        return_vals: Arc<Mutex<Vec<Result<SuccessStatus, WithdrawerError>>>>,
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

        pub(crate) fn to_return(&self, result: Result<SuccessStatus, WithdrawerError>) {
            self.return_vals.lock().unwrap().push(result);
        }
    }

    impl Withdrawer for MockWithdrawer {
        fn withdraw(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            amount: Amount,
        ) -> Result<SuccessStatus, WithdrawerError> {
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

    use crate::account::account_transactor::SuccessStatus;
    use crate::{
        account::{
            account_transactor::SuccessStatus::Duplicate,
            account_transactor::SuccessStatus::Transacted,
            transactors::withdrawer::WithdrawerError::InsufficientFund,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Withdrawal, WithdrawalStatus,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::WithdrawerError;

    use super::SimpleWithdrawer;
    use super::Withdrawer;

    #[rstest]
    //    |-------------------- input -----------------------------| |------------------------------- output ----------------------------------|
    //                                            tx
    //     original_account,                      id,                                expected_account
    //        avail, existing withdrawals,            amount, expected_status           avail, existing withdrawals
    #[case(active(7, vec![]),                      0,      8, Err(InsufficientFund), active(7, vec![])                                          )]
    #[case(active(7, vec![]),                      0,      0, Ok(Transacted),        active(7, vec![(0, accepted_wdr(0))])                      )]
    #[case(active(7, vec![]),                      0,      4, Ok(Transacted),        active(3, vec![(0, accepted_wdr(4))])                      )]
    #[case(active(7, vec![]),                      0,      7, Ok(Transacted),        active(0, vec![(0, accepted_wdr(7))])                      )]
    #[case(active(7, vec![(0, accepted_wdr(3))]),  0,      3, Ok(Duplicate),         active(7, vec![(0, accepted_wdr(3))])                      )]
    #[case(active(7, vec![(0, accepted_wdr(3))]),  1,      5, Ok(Transacted),        active(2, vec![(0, accepted_wdr(3)), (1, accepted_wdr(5))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] amount_i64: i64,
        #[case] expected_status: Result<SuccessStatus, WithdrawerError>,
        #[case] expected: Account,
    ) {
        let withdrawer = SimpleWithdrawer;
        assert_eq!(
            withdrawer.withdraw(&mut original, transaction_id, amount(amount_i64)),
            expected_status
        );
        assert_eq!(original, expected);
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

    fn active(available: i64, withdrawals: Vec<(TransactionId, Withdrawal)>) -> Account {
        account(Active, available, 0, withdrawals)
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
