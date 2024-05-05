use thiserror::Error;

use crate::{
    account::Account,
    model::{Transaction, TransactionId, TransactionKind},
};

use super::transactors::{
    depositor::{Depositor, DepositorError, SimpleDepositor},
    disputer::{CreditDebitDisputer, Disputer, DisputerError},
    resolver::{CreditDebitResolver, Resolver, ResolverError},
    withdrawer::{SimpleWithdrawer, Withdrawer, WithdrawerError},
};

pub trait AccountTransactor {
    fn transact(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactorError>;
}

pub struct SimpleAccountTransactor {
    depositor: Box<dyn Depositor + Send + Sync>,
    withdrawer: Box<dyn Withdrawer + Send + Sync>,
    disputer: Box<dyn Disputer + Send + Sync>,
    resolver: Box<dyn Resolver + Send + Sync>,
}

impl AccountTransactor for SimpleAccountTransactor {
    fn transact(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactorError> {
        let Transaction {
            transaction_id,
            kind,
            client_id: _,
        } = transaction;
        match kind {
            TransactionKind::Deposit { amount } => {
                self.depositor.deposit(account, transaction_id, amount)?
            }
            TransactionKind::Withdrawal { amount } => {
                self.withdrawer.withdraw(account, transaction_id, amount)?
            }
            TransactionKind::Dispute => self.disputer.dispute(account, transaction_id)?,
            TransactionKind::Resolve => self.resolver.resolve(account, transaction_id)?,
            TransactionKind::ChargeBack => todo!(),
        }
        Ok(())
    }
}

impl SimpleAccountTransactor {
    pub fn new() -> Self {
        let depositor = SimpleDepositor;
        let withdrawer = SimpleWithdrawer;
        let disputer = CreditDebitDisputer;
        let resolver = CreditDebitResolver;

        Self {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
            resolver: Box::new(resolver),
        }
    }
}

/// TODO: collapse them into a general one that carries the internal error
/// from each processor.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum AccountTransactorError {
    /// TODO: can i provide more info here?
    #[error("Mismatch")]
    MismatchTransactionKind,

    #[error("Depositing to a locked account is not allowed.")]
    CannotDepositToLockedAccount,
    #[error("Withdrawing from a locked account is not allowed.")]
    CannotWithdrawFromLockedAccount,
    #[error("There is insufficient fund in the account for the withdrawal requested.")]
    InsufficientFundForWithdrawal,
    #[error("Disputing against a locked account is not allowed.")]
    CannotDisputeAgainstLockedAccount,
    #[error("The target transaction was not found.")]
    NoTransactionFound,
    #[error("Resolving a locked account is not allowed.")]
    CannotResolveLockedAccount,
    #[error("Resolving a non disputed transaction is not allowed: {0}")]
    CannotResolveNonDisputedTransaction(TransactionId),
}

impl From<DepositorError> for AccountTransactorError {
    fn from(err: DepositorError) -> Self {
        match err {
            DepositorError::AccountLocked => Self::CannotDepositToLockedAccount,
        }
    }
}

impl From<WithdrawerError> for AccountTransactorError {
    fn from(err: WithdrawerError) -> Self {
        match err {
            WithdrawerError::AccountLocked => Self::CannotWithdrawFromLockedAccount,
            WithdrawerError::InsufficientFund => Self::InsufficientFundForWithdrawal,
        }
    }
}

impl From<DisputerError> for AccountTransactorError {
    fn from(err: DisputerError) -> Self {
        match err {
            DisputerError::AccountLocked => Self::CannotDisputeAgainstLockedAccount,
            DisputerError::NoTransactionFound => Self::NoTransactionFound,
        }
    }
}

impl From<ResolverError> for AccountTransactorError {
    fn from(err: ResolverError) -> Self {
        match err {
            ResolverError::AccountLocked => Self::CannotResolveLockedAccount,
            ResolverError::CannotResoveNonDisputedTransaction(txn_id) => {
                Self::CannotResolveNonDisputedTransaction(txn_id)
            }
            ResolverError::NoTransactionFound => Self::NoTransactionFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        account::{
            transactors::{
                depositor::{mock::MockDepositor, DepositorError},
                disputer::{mock::MockDisputer, DisputerError},
                resolver::{mock::MockResolver, ResolverError},
                withdrawer::{mock::MockWithdrawer, WithdrawerError},
            },
            Account, AccountSnapshot, AccountStatus,
        },
        model::{Amount, ClientId, Transaction, TransactionId, TransactionKind},
    };

    use super::{AccountTransactor, AccountTransactorError, SimpleAccountTransactor};

    impl SimpleAccountTransactor {
        fn new_for_test(
            depositor: MockDepositor,
            withdrawer: MockWithdrawer,
            disputer: MockDisputer,
            resolver: MockResolver,
        ) -> Self {
            Self {
                depositor: Box::new(depositor),
                withdrawer: Box::new(withdrawer),
                disputer: Box::new(disputer),
                resolver: Box::new(resolver),
            }
        }
    }
    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        depositor.expect(&mut account, transaction_id, amount);
        depositor.to_return(Ok(()));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);
        processor.transact(&mut account, deposit(0, 0)).unwrap();
    }

    #[rstest]
    #[case(
        DepositorError::AccountLocked,
        AccountTransactorError::CannotDepositToLockedAccount
    )]
    fn error_returned_from_depositor_is_propagated(
        #[case] depositor_error: DepositorError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        depositor.expect(&mut account.clone(), transaction_id, amount);
        depositor.to_return(Err(depositor_error));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);

        assert_eq!(
            processor.transact(&mut account, deposit(0, 0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_withdrawer_for_withdrawal() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        withdrawer.expect(&mut account, transaction_id, amount);
        withdrawer.to_return(Ok(()));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);
        processor.transact(&mut account, withdrawal(0, 0)).unwrap();
    }

    #[rstest]
    #[case(
        WithdrawerError::AccountLocked,
        AccountTransactorError::CannotWithdrawFromLockedAccount
    )]
    #[case(
        WithdrawerError::InsufficientFund,
        AccountTransactorError::InsufficientFundForWithdrawal
    )]
    fn error_returned_from_withdrawer_is_propagated(
        #[case] withdrawer_error: WithdrawerError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        withdrawer.expect(&mut account.clone(), transaction_id, amount);
        withdrawer.to_return(Err(withdrawer_error));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);

        assert_eq!(
            processor.transact(&mut account, withdrawal(0, 0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_disputer_for_dispute() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        disputer.expect(&mut account, transaction_id);
        disputer.to_return(Ok(()));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);
        processor.transact(&mut account, dispute(0)).unwrap();
    }

    #[rstest]
    #[case(
        DisputerError::AccountLocked,
        AccountTransactorError::CannotDisputeAgainstLockedAccount
    )]
    #[case(
        DisputerError::NoTransactionFound,
        AccountTransactorError::NoTransactionFound
    )]
    fn error_returned_from_disputer_is_propagated(
        #[case] disputer_error: DisputerError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        disputer.expect(&mut account.clone(), transaction_id);
        disputer.to_return(Err(disputer_error));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);

        assert_eq!(
            processor.transact(&mut account, dispute(0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_resolver_for_resolve() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        resolver.expect(&mut account, transaction_id);
        resolver.to_return(Ok(()));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);
        processor.transact(&mut account, resolve(0)).unwrap();
    }

    #[rstest]
    #[case(
        ResolverError::AccountLocked,
        AccountTransactorError::CannotResolveLockedAccount
    )]
    #[case(
        ResolverError::NoTransactionFound,
        AccountTransactorError::NoTransactionFound
    )]
    #[case(
        ResolverError::CannotResoveNonDisputedTransaction(0),
        AccountTransactorError::CannotResolveNonDisputedTransaction(0)
    )]
    fn error_returned_from_resolver_is_propagated(
        #[case] disputer_error: ResolverError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        resolver.expect(&mut account.clone(), transaction_id);
        resolver.to_return(Err(disputer_error));
        let processor =
            SimpleAccountTransactor::new_for_test(depositor, withdrawer, disputer, resolver);

        assert_eq!(
            processor.transact(&mut account, resolve(0)),
            Err(expected_error)
        );
    }

    fn some_account() -> Account {
        Account {
            client_id: 1234,
            status: AccountStatus::Active,
            account_snapshot: AccountSnapshot::empty(),
            deposits: HashMap::new(),
            withdrawals: HashMap::new(),
        }
    }

    fn deposit(transaction_id: TransactionId, amount: u32) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind: TransactionKind::Deposit {
                amount: OrderedFloat(amount as f32),
            },
        }
    }

    fn withdrawal(transaction_id: TransactionId, amount: u32) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind: TransactionKind::Withdrawal {
                amount: OrderedFloat(amount as f32),
            },
        }
    }

    fn dispute(transaction_id: TransactionId) -> Transaction {
        transaction(transaction_id, TransactionKind::Dispute)
    }

    fn resolve(transaction_id: TransactionId) -> Transaction {
        transaction(transaction_id, TransactionKind::Resolve)
    }

    fn transaction(transaction_id: TransactionId, kind: TransactionKind) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind,
        }
    }
}
