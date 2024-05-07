use crate::{
    account::{account_transactor::SuccessStatus, Account, AccountStatus, DepositStatus},
    model::TransactionId,
};

use super::{Resolver, ResolverError};

pub(crate) struct CreditResolver;

impl Resolver for CreditResolver {
    fn resolve(
        &self,
        account: &mut Account,
        transaction_id: TransactionId,
    ) -> Result<SuccessStatus, ResolverError> {
        if account.status == AccountStatus::Locked {
            return Err(ResolverError::AccountLocked);
        }
        match account.deposits.get_mut(&transaction_id) {
            Some(deposit) => match deposit.status {
                DepositStatus::Held => {
                    account.account_snapshot.available.0 += deposit.amount.0;
                    account.account_snapshot.held.0 -= deposit.amount.0;
                    deposit.status = DepositStatus::Resolved;
                    return Ok(SuccessStatus::Transacted);
                }
                DepositStatus::Resolved => return Ok(SuccessStatus::Duplicate),
                _ => return Err(ResolverError::NonDisputedTransaction),
            },
            None => Err(ResolverError::NoTransactionFound),
        }
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use crate::account::transactors::resolver::ResolverError;
    use crate::{
        account::{
            account_transactor::SuccessStatus,
            account_transactor::SuccessStatus::Duplicate,
            account_transactor::SuccessStatus::Transacted,
            transactors::resolver::ResolverError::NoTransactionFound,
            transactors::resolver::ResolverError::NonDisputedTransaction,
            Account, AccountSnapshot,
            AccountStatus::{self, Active, Locked},
            Deposit, DepositStatus, Withdrawal,
        },
        model::{Amount, Amount4DecimalBased, TransactionId},
    };

    use super::CreditResolver;
    use super::Resolver;

    #[rstest]
    #[rustfmt::skip(case)]
    // disputing credit transactions
    //    |------------------ input -----------------------| |----------------------------- output ------------------------------------|
    //     original_account,                            tx                               expected_account
    //        avail, held, deposits,                    id, expected_status,                 avail, held, deposits
    #[case(active(7,    5, vec![(0, held_dep(3))]),      0, Ok(Transacted),              active(10,    2, vec![(0, resolved_dep(3))]) )]
    #[case(active(7,    0, vec![(0, resolved_dep(3))]),  0, Ok(Duplicate),               active( 7,    0, vec![(0, resolved_dep(3))]) )]
    #[case(active(7,    0, vec![(0, accepted_dep(3))]),  0, Err(NonDisputedTransaction), active( 7,    0, vec![(0, accepted_dep(3))]) )]
    #[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 0, Err(NonDisputedTransaction), active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
    #[case(active(7,    0, vec![(0, chrgd_bck_dep(3))]), 1, Err(NoTransactionFound),     active( 7,    0, vec![(0, chrgd_bck_dep(3))]))]
    fn active_account_cases(
        #[case] mut original: Account,
        #[case] transaction_id: TransactionId,
        #[case] expected_status: Result<SuccessStatus, ResolverError>,
        #[case] expected: Account,
    ) {
        let resolver = CreditResolver;
        assert_eq!(
            resolver.resolve(&mut original, transaction_id),
            expected_status
        );
        assert_eq!(original, expected);
    }

    #[test]
    fn returns_bad_transaction_when_no_matching_transaction() {
        let mut account = active(100, 110, vec![(0, accepted_dep(2))]);
        let resolver = CreditResolver;
        assert_eq!(
            resolver.resolve(&mut account, 1),
            Err(ResolverError::NoTransactionFound)
        );
    }

    #[test]
    fn resolving_a_locked_account_returns_error() {
        let mut account = account(Locked, 0, 0, vec![], vec![]);
        let resolver = CreditResolver;
        assert_eq!(
            resolver.resolve(&mut account, 0),
            Err(ResolverError::AccountLocked)
        );
    }

    fn active(available: i64, held: i64, deposits: Vec<(TransactionId, Deposit)>) -> Account {
        account(Active, available, held, deposits, vec![])
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
