use crate::{
    account::{account_transactor::SuccessStatus, Account, AccountStatus, DepositStatus},
    model::TransactionId,
};

use super::{Disputer, DisputerError};

pub(crate) struct CreditDisputer;

impl Disputer for CreditDisputer {
    fn dispute(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<SuccessStatus, DisputerError> {
        match account.deposits.get_mut(&transaction_id) {
            Some(deposit) => match deposit.status {
                DepositStatus::Accepted => {
                    if account.status == AccountStatus::Locked {
                        return Err(DisputerError::AccountLocked);
                    }
                    account.account_snapshot.available.0 -= deposit.amount.0;
                    account.account_snapshot.held.0 += deposit.amount.0;
                    deposit.status = DepositStatus::Held;
                    return Ok(SuccessStatus::Transacted);
                }
                _ => return Ok(SuccessStatus::Duplicate),
            },
            None => {
                if account.status == AccountStatus::Locked {
                    return Err(DisputerError::AccountLocked);
                }
                Err(DisputerError::NoTransactionFound)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use crate::{
        account::{
            account_transactor::SuccessStatus,
            account_transactor::SuccessStatus::Duplicate,
            account_transactor::SuccessStatus::Transacted,
            transactors::disputer::DisputerError,
            transactors::disputer::DisputerError::AccountLocked,
            transactors::disputer::DisputerError::NoTransactionFound,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus, Withdrawal,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::CreditDisputer;
    use super::Disputer;

    #[rstest]
    #[rustfmt::skip(case)]
    // disputing credit transactions
    //    |------------------ input ---------------------| |------------------------- output -----------------------------------|
    //     original_account,                            tx                           expected_account
    //        avail, held, deposits,                    id, expected_status,             avail, held, deposits
    #[case(active(7,    0, vec![(0, accepted_dep(3))] ), 0, Ok(Transacted),          active( 4,    3, vec![(0, held_dep(3))]     ))]
    #[case(active(7,    0, vec![(0, held_dep(3))]     ), 0, Ok(Duplicate),           active( 7,    0, vec![(0, held_dep(3))]     ))]
    #[case(active(7,    0, vec![(0, resolved_dep(3))] ), 0, Ok(Duplicate),           active( 7,    0, vec![(0, resolved_dep(3))] ))]
    #[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),           active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
    #[case(active(3,    0, vec![(0, accepted_dep(7))] ), 0, Ok(Transacted),          active(-4,    7, vec![(0, held_dep(7))]     ))]
    #[case(active(3,    0, vec![(0, accepted_dep(7))] ), 1, Err(NoTransactionFound), active( 3,    0, vec![(0, accepted_dep(7))] ))]
    // locked cases
    #[case(locked(7,    0, vec![(0, accepted_dep(3))] ), 0, Err(AccountLocked),      locked( 7,    0, vec![(0, accepted_dep(3))] ))]
    #[case(locked(7,    0, vec![(0, accepted_dep(3))] ), 1, Err(AccountLocked),      locked( 7,    0, vec![(0, accepted_dep(3))] ))]
    #[case(locked(7,    0, vec![(0, held_dep(3))]     ), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, held_dep(3))]     ))]
    #[case(locked(7,    0, vec![(0, resolved_dep(3))] ), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, resolved_dep(3))] ))]
    #[case(locked(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Ok(Duplicate),           locked( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected_status: Result<SuccessStatus, DisputerError>,
        #[case] expected: Account,
    ) {
        let disputer = CreditDisputer;
        assert_eq!(
            disputer.dispute(&mut original, transaction_id),
            expected_status
        );
        assert_eq!(original, expected);
    }

    fn active(available: i64, held: i64, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Active, available, held, deposits, vec![])
    }

    fn locked(available: i64, held: i64, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Locked, available, held, deposits, vec![])
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

    fn amount(amount: i64) -> Amount {
        Amount4DecimalBased(amount)
    }
}
