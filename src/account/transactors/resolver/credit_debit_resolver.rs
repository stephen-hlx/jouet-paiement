use crate::{
    account::{Account, AccountStatus, DepositStatus, WithdrawalStatus},
    model::TransactionId,
};

use super::{Resolver, ResolverError};

pub(crate) struct CreditDebitResolver;

impl Resolver for CreditDebitResolver {
    fn resolve(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<(), ResolverError> {
        if let Some(deposit) = account.deposits.get_mut(&transaction_id) {
            match deposit.status {
                DepositStatus::Accepted => {
                    if account.status == AccountStatus::Locked {
                        return Err(ResolverError::AccountLocked);
                    }
                    account.account_snapshot.available -= deposit.amount;
                    account.account_snapshot.held += deposit.amount;
                    deposit.status = DepositStatus::Held;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        } else if let Some(withdrawal) = account.withdrawals.get_mut(&transaction_id) {
            match withdrawal.status {
                WithdrawalStatus::Accepted => {
                    if account.status == AccountStatus::Locked {
                        return Err(ResolverError::AccountLocked);
                    }
                    withdrawal.status = WithdrawalStatus::Held;
                    account.account_snapshot.available += withdrawal.amount;
                    account.account_snapshot.held -= withdrawal.amount;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }
        Err(ResolverError::NoTransactionFound)
    }
}

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;
    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        account::{
            transactors::resolver::ResolverError,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus, Withdrawal, WithdrawalStatus,
        },
        model::{Amount, TransactionId},
    };

    use super::CreditDebitResolver;
    use super::Resolver;

    #[rstest]
    #[rustfmt::skip(case)]
    //    |------------------ input ------------------------------| |-------------- output ------------------------|
    //                                                           tx
    //     original_account,                                     id,   expected_account
    //         avail, held, deposits,                 withdraws,          avail, held, deposits,               withdrawals
    #[case(active( 7,    0, vec![(0, accepted_dep(3))],  vec![]), 0,  active( 4,    3, vec![(0, held_dep(3))],      vec![]))]
    #[case(active( 7,    0, vec![(0, held_dep(3))],      vec![]), 0,  active( 7,    0, vec![(0, held_dep(3))],      vec![]))]
    #[case(active( 7,    0, vec![(0, resolved_dep(3))],  vec![]), 0,  active( 7,    0, vec![(0, resolved_dep(3))],  vec![]))]
    #[case(active( 7,    0, vec![(0, chrgd_bck_dep(3))], vec![]), 0,  active( 7,    0, vec![(0, chrgd_bck_dep(3))], vec![]))]
    //    |------------------ input ------------------------------| |-------------- output ------------------------|
    //                                                           tx
    //     original_account,                                     id,   expected_account
    //         avail, held, deposits, withdraws,                         avail,  held, deposits, withdrawals
    #[case(active( 7,    3, vec![], vec![(0, accepted_wdr(3))]),  0,  active(10,    0, vec![], vec![(0, held_wdr(3))])     )]
    #[case(active( 7,    0, vec![], vec![(0, accepted_wdr(3))]),  0,  active(10,   -3, vec![], vec![(0, held_wdr(3))])     )]
    #[case(active( 7,    0, vec![], vec![(0, held_wdr(3))]),      0,  active( 7,    0, vec![], vec![(0, held_wdr(3))])     )]
    #[case(active( 7,    0, vec![], vec![(0, resolved_wdr(3))]),  0,  active( 7,    0, vec![], vec![(0, resolved_wdr(3))]) )]
    #[case(active( 7,    0, vec![], vec![(0, chrgd_bck_wdr(3))]), 0,  active( 7,    0, vec![], vec![(0, chrgd_bck_wdr(3))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected: Account,
    ) {
        let resolver = CreditDebitResolver;
        resolver.resolve(&mut original, transaction_id).unwrap();
        assert_eq!(original, expected);
    }

    #[test]
    fn returns_bad_transaction_when_no_matching_transaction() {
        let mut account = active(
            100,
            110,
            vec![(1, accepted_dep(2))],
            vec![(3, accepted_wdr(4))],
        );
        let resolver = CreditDebitResolver;
        assert_matches!(
            resolver.resolve(&mut account, 0),
            Err(ResolverError::NoTransactionFound)
        );
    }

    #[rstest]
    //    |---------------------------- input -------------------------------| |------------ output -------------------|
    //            deposits,                    withdrawals,                 tx, result
    #[case(locked(vec![(1, accepted_dep(2))],  vec![(3, accepted_wdr(4))]),  0, Err(ResolverError::NoTransactionFound))]
    #[case(locked(vec![(1, accepted_dep(2))],  vec![(3, accepted_wdr(4))]),  1, Err(ResolverError::AccountLocked)     )]
    #[case(locked(vec![(1, accepted_dep(2))],  vec![(3, accepted_wdr(4))]),  3, Err(ResolverError::AccountLocked)     )]
    #[case(locked(vec![(1, held_dep(2))],      vec![(3, held_wdr(4))]),      1, Ok(())                                )]
    #[case(locked(vec![(1, held_dep(2))],      vec![(3, held_wdr(4))]),      3, Ok(())                                )]
    #[case(locked(vec![(1, resolved_dep(2))],  vec![(3, resolved_wdr(4))]),  1, Ok(())                                )]
    #[case(locked(vec![(1, resolved_dep(2))],  vec![(3, resolved_wdr(4))]),  3, Ok(())                                )]
    #[case(locked(vec![(1, chrgd_bck_dep(2))], vec![(3, chrgd_bck_wdr(4))]), 1, Ok(())                                )]
    #[case(locked(vec![(1, chrgd_bck_dep(2))], vec![(3, chrgd_bck_wdr(4))]), 3, Ok(())                                )]
    fn locked_account_case(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected: Result<(), ResolverError>,
    ) {
        let resolver = CreditDebitResolver;
        assert_eq!(resolver.resolve(&mut original, transaction_id), expected);
    }

    fn locked(
        deposits: Vec<(TransactionId, Deposit)>,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        account(Locked, 0, 0, deposits, withdrawals)
    }
    fn active(
        available: i32,
        held: i32,
        deposits: Vec<(TransactionId, Deposit)>,
        withdrawals: Vec<(TransactionId, Withdrawal)>,
    ) -> Account {
        account(Active, available, held, deposits, withdrawals)
    }

    fn account(
        status: AccountStatus,
        available: i32,
        held: i32,
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

    fn accepted_dep(amount_i32: i32) -> Deposit {
        deposit(amount_i32, DepositStatus::Accepted)
    }

    fn held_dep(amount_i32: i32) -> Deposit {
        deposit(amount_i32, DepositStatus::Held)
    }

    fn resolved_dep(amount_i32: i32) -> Deposit {
        deposit(amount_i32, DepositStatus::Resolved)
    }

    fn chrgd_bck_dep(amount_i32: i32) -> Deposit {
        deposit(amount_i32, DepositStatus::ChargedBack)
    }

    fn deposit(amount_i32: i32, status: DepositStatus) -> Deposit {
        Deposit {
            amount: amount(amount_i32),
            status,
        }
    }

    fn accepted_wdr(amount_i32: i32) -> Withdrawal {
        withdrawal(amount_i32, WithdrawalStatus::Accepted)
    }

    fn held_wdr(amount_i32: i32) -> Withdrawal {
        withdrawal(amount_i32, WithdrawalStatus::Held)
    }

    fn resolved_wdr(amount_i32: i32) -> Withdrawal {
        withdrawal(amount_i32, WithdrawalStatus::Resolved)
    }

    fn chrgd_bck_wdr(amount_i32: i32) -> Withdrawal {
        withdrawal(amount_i32, WithdrawalStatus::ChargedBack)
    }

    fn withdrawal(amount_u32: i32, status: WithdrawalStatus) -> Withdrawal {
        Withdrawal {
            amount: amount(amount_u32),
            status,
        }
    }

    fn amount(amount: i32) -> Amount {
        OrderedFloat(amount as f32)
    }
}
