use crate::{
    account::{Account, AccountStatus, DepositStatus, WithdrawalStatus},
    model::TransactionId,
};

use super::{Backcharger, BackchargerError};

pub(crate) struct CreditDebitBackcharger;

impl Backcharger for CreditDebitBackcharger {
    fn chargeback(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<(), BackchargerError> {
        if let Some(deposit) = account.deposits.get_mut(&transaction_id) {
            match deposit.status {
                DepositStatus::Held => {
                    if account.status == AccountStatus::Locked {
                        return Err(BackchargerError::AccountLocked);
                    }
                    account.account_snapshot.held.0 -= deposit.amount.0;
                    deposit.status = DepositStatus::ChargedBack;
                    account.status = AccountStatus::Locked;
                    return Ok(());
                }
                DepositStatus::ChargedBack => return Ok(()),
                _ => {
                    return Err(BackchargerError::CannotChargebackNonDisputedTransaction(
                        transaction_id,
                    ))
                }
            }
        } else if let Some(withdrawal) = account.withdrawals.get_mut(&transaction_id) {
            match withdrawal.status {
                WithdrawalStatus::Held => {
                    if account.status == AccountStatus::Locked {
                        return Err(BackchargerError::AccountLocked);
                    }
                    account.account_snapshot.held.0 += withdrawal.amount.0;
                    withdrawal.status = WithdrawalStatus::ChargedBack;
                    account.status = AccountStatus::Locked;
                    return Ok(());
                }
                WithdrawalStatus::ChargedBack => return Ok(()),
                _ => {
                    return Err(BackchargerError::CannotChargebackNonDisputedTransaction(
                        transaction_id,
                    ))
                }
            }
        }
        Err(BackchargerError::NoTransactionFound)
    }
}

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;
    use rstest::rstest;

    use crate::{
        account::{
            transactors::backcharger::BackchargerError,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus, Withdrawal, WithdrawalStatus,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::Backcharger;
    use super::CreditDebitBackcharger;

    #[rstest]
    #[rustfmt::skip(case)]
    // disputing credit transactions
    //    |------------------ input ------------------------------| |-------------------- output ------------------------|
    //                                                           tx
    //     original_account,                                     id,   expected_account
    //        avail, held, deposits,                 withdraws,         avail, held, deposits,                withdrawals
    #[case(active(7,    5, vec![(0, held_dep(3))],      vec![]), 0,  locked(7,    2, vec![(0, chrgd_bck_dep(3))],  vec![]))]
    // disputing debit transactions
    //    |------------------ input ------------------------------| |-------------------- output ------------------------|
    //                                                           tx
    //     original_account,                                     id,   expected_account
    //        avail, held, deposits, withdraws,                         avail, held, deposits, withdrawals
    #[case(active(7,    5, vec![], vec![(0, held_wdr(3))]),      0,  locked(7,    8, vec![], vec![(0, chrgd_bck_wdr(3))]) )]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected: Account,
    ) {
        let resolver = CreditDebitBackcharger;
        resolver.chargeback(&mut original, transaction_id).unwrap();
        assert_eq!(original, expected);
    }

    #[rstest]
    #[rustfmt::skip(case)]
    // disputing credit transactions
    //     original_account,                                     tx
    //        avail, held, deposits,                 withdraws,  id,
    #[case(active(0,    0, vec![(0, accepted_dep(3))],  vec![]), 0)]
    #[case(active(0,    0, vec![(0, resolved_dep(3))],  vec![]), 0)]
    // disputing debit transactions
    //    |------------------ input ------------------------------|
    //     original_account,                                     tx
    //        avail, held, deposits, withdraws,                  id,
    #[case(active(0,    0, vec![], vec![(0, accepted_wdr(3))]),  0,)]
    #[case(active(0,    0, vec![], vec![(0, resolved_wdr(3))]),  0,)]
    fn non_backchargeable_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
    ) {
        let resolver = CreditDebitBackcharger;
        assert_matches!(
            resolver.chargeback(&mut original, transaction_id),
            Err(BackchargerError::CannotChargebackNonDisputedTransaction(0))
        );
    }

    #[test]
    fn returns_error_when_no_matching_transaction() {
        let mut account = active(
            100,
            110,
            vec![(1, accepted_dep(2))],
            vec![(3, accepted_wdr(4))],
        );
        let resolver = CreditDebitBackcharger;
        assert_matches!(
            resolver.chargeback(&mut account, 0),
            Err(BackchargerError::NoTransactionFound)
        );
    }

    #[rstest]
    //    |---------------------------- input --------------------------------------| |--------------- output -----------------|
    //                  deposits,                    withdrawals,                 tx, result
    #[case(locked(0, 0, vec![(1, held_dep(2))],      vec![(3, held_wdr(4))]),      0, Err(BackchargerError::NoTransactionFound))]
    #[case(locked(0, 0, vec![(1, held_dep(2))],      vec![(3, held_wdr(4))]),      1, Err(BackchargerError::AccountLocked)     )]
    #[case(locked(0, 0, vec![(1, held_dep(2))],      vec![(3, held_wdr(4))]),      3, Err(BackchargerError::AccountLocked)     )]
    #[case(locked(0, 0, vec![(1, chrgd_bck_dep(2))], vec![(3, chrgd_bck_wdr(4))]), 1, Ok(())                                   )]
    #[case(locked(0, 0, vec![(1, chrgd_bck_dep(2))], vec![(3, chrgd_bck_wdr(4))]), 3, Ok(())                                   )]
    fn locked_account_case(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected: Result<(), BackchargerError>,
    ) {
        let resolver = CreditDebitBackcharger;
        assert_eq!(resolver.chargeback(&mut original, transaction_id), expected);
    }

    fn locked(
        available: i64,
        held: i64,
        deposits: Vec<(TransactionId, Deposit)>,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        account(Locked, available, held, deposits, withdrawals)
    }
    fn active(
        available: i64,
        held: i64,
        deposits: Vec<(TransactionId, Deposit)>,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        account(Active, available, held, deposits, withdrawals)
    }

    fn account(
        status: AccountStatus,
        available: i64,
        held: i64,
        deposits: Vec<(TransactionId, Deposit)>,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        Account {
            client_id: 1234,
            status,
            account_snapshot: AccountSnapshot::new(available, held),
            deposits: deposits.into_iter().collect(),
            withdrawals: withdrawals.into_iter().collect(),
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

    fn withdrawal(amount_u32: i64, status: WithdrawalStatus) -> Withdrawal {
        Withdrawal {
            amount: amount(amount_u32),
            status,
        }
    }

    fn amount(amount: i64) -> Amount {
        Amount4DecimalBased(amount)
    }
}
