use thiserror::Error;

use crate::{
    account::Account,
    model::{Transaction, TransactionKind},
};

use super::transactors::{
    backcharger::{Backcharger, BackchargerError, CreditBackcharger},
    depositor::{Depositor, DepositorError, SimpleDepositor},
    disputer::{CreditDisputer, Disputer, DisputerError},
    resolver::{CreditResolver, Resolver, ResolverError},
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
    backcharger: Box<dyn Backcharger + Send + Sync>,
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
                let _status = self.depositor.deposit(account, transaction_id, amount)?;
            }
            TransactionKind::Withdrawal { amount } => {
                let _status = self.withdrawer.withdraw(account, transaction_id, amount)?;
            }
            TransactionKind::Dispute => {
                let _status = self.disputer.dispute(account, transaction_id)?;
            }
            TransactionKind::Resolve => {
                let _status = self.resolver.resolve(account, transaction_id)?;
            }
            TransactionKind::ChargeBack => {
                let _status = self.backcharger.chargeback(account, transaction_id)?;
            }
        }
        Ok(())
    }
}

impl SimpleAccountTransactor {
    pub fn new() -> Self {
        let depositor = SimpleDepositor;
        let withdrawer = SimpleWithdrawer;
        let disputer = CreditDisputer;
        let resolver = CreditResolver;
        let backcharger = CreditBackcharger;

        Self {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
            resolver: Box::new(resolver),
            backcharger: Box::new(backcharger),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SuccessStatus {
    Transacted,
    Duplicate,
}

#[derive(Debug, Error, PartialEq, Clone)]
pub enum AccountTransactorError {
    #[error("The account is locked")]
    AccountLocked,

    #[error("The transaction is incompatible")]
    IncompatibleTransaction,

    #[error("Insufficient fund for withdrawal")]
    InsufficientFundForWithdrawal,

    #[error("No tranasction found")]
    NoTransactionFound,
}

impl From<DepositorError> for AccountTransactorError {
    fn from(err: DepositorError) -> Self {
        match err {
            DepositorError::AccountLocked => Self::AccountLocked,
        }
    }
}

impl From<WithdrawerError> for AccountTransactorError {
    fn from(err: WithdrawerError) -> Self {
        match err {
            WithdrawerError::AccountLocked => Self::AccountLocked,
            WithdrawerError::InsufficientFund => Self::InsufficientFundForWithdrawal,
        }
    }
}

impl From<DisputerError> for AccountTransactorError {
    fn from(err: DisputerError) -> Self {
        match err {
            DisputerError::AccountLocked => Self::AccountLocked,
            DisputerError::NoTransactionFound => Self::NoTransactionFound,
        }
    }
}

impl From<ResolverError> for AccountTransactorError {
    fn from(err: ResolverError) -> Self {
        match err {
            ResolverError::AccountLocked => Self::AccountLocked,
            ResolverError::NonDisputedTransaction => Self::IncompatibleTransaction,
            ResolverError::NoTransactionFound => Self::NoTransactionFound,
        }
    }
}

impl From<BackchargerError> for AccountTransactorError {
    fn from(err: BackchargerError) -> Self {
        match err {
            BackchargerError::AccountLocked => Self::AccountLocked,
            BackchargerError::NoTransactionFound => Self::NoTransactionFound,
            BackchargerError::NonDisputedTransaction => Self::IncompatibleTransaction,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use crate::{
        account::{
            transactors::{
                backcharger::{mock::MockBackcharger, BackchargerError},
                depositor::{mock::MockDepositor, DepositorError},
                disputer::{mock::MockDisputer, DisputerError},
                resolver::{mock::MockResolver, ResolverError},
                withdrawer::{mock::MockWithdrawer, WithdrawerError},
            },
            Account, AccountSnapshot, AccountStatus,
        },
        model::{
            Amount, Amount4DecimalBased, ClientId, Transaction, TransactionId, TransactionKind,
        },
    };

    use super::{
        AccountTransactor, AccountTransactorError, SimpleAccountTransactor, SuccessStatus,
    };

    impl SimpleAccountTransactor {
        fn new_for_test(
            depositor: MockDepositor,
            withdrawer: MockWithdrawer,
            disputer: MockDisputer,
            resolver: MockResolver,
            backcharger: MockBackcharger,
        ) -> Self {
            Self {
                depositor: Box::new(depositor),
                withdrawer: Box::new(withdrawer),
                disputer: Box::new(disputer),
                resolver: Box::new(resolver),
                backcharger: Box::new(backcharger),
            }
        }
    }
    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = Amount4DecimalBased(0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        depositor.expect(&mut account, transaction_id, amount);
        depositor.to_return(Ok(SuccessStatus::Transacted));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );
        processor.transact(&mut account, deposit(0, 0)).unwrap();
    }

    #[rstest]
    #[case(DepositorError::AccountLocked, AccountTransactorError::AccountLocked)]
    fn error_returned_from_depositor_is_propagated(
        #[case] depositor_error: DepositorError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = Amount4DecimalBased(0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        depositor.expect(&mut account.clone(), transaction_id, amount);
        depositor.to_return(Err(depositor_error));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );

        assert_eq!(
            processor.transact(&mut account, deposit(0, 0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_withdrawer_for_withdrawal() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = Amount4DecimalBased(0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        withdrawer.expect(&mut account, transaction_id, amount);
        withdrawer.to_return(Ok(SuccessStatus::Transacted));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );
        processor.transact(&mut account, withdrawal(0, 0)).unwrap();
    }

    #[rstest]
    #[case(WithdrawerError::AccountLocked, AccountTransactorError::AccountLocked)]
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
        let amount: Amount = Amount4DecimalBased(0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        withdrawer.expect(&mut account.clone(), transaction_id, amount);
        withdrawer.to_return(Err(withdrawer_error));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );

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
        let backcharger = MockBackcharger::new();
        disputer.expect(&mut account, transaction_id);
        disputer.to_return(Ok(SuccessStatus::Transacted));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );
        processor.transact(&mut account, dispute(0)).unwrap();
    }

    #[rstest]
    #[case(DisputerError::AccountLocked, AccountTransactorError::AccountLocked)]
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
        let backcharger = MockBackcharger::new();
        disputer.expect(&mut account.clone(), transaction_id);
        disputer.to_return(Err(disputer_error));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );

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
        let backcharger = MockBackcharger::new();
        resolver.expect(&mut account, transaction_id);
        resolver.to_return(Ok(SuccessStatus::Transacted));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );
        processor.transact(&mut account, resolve(0)).unwrap();
    }

    #[rstest]
    #[case(ResolverError::AccountLocked, AccountTransactorError::AccountLocked)]
    #[case(
        ResolverError::NoTransactionFound,
        AccountTransactorError::NoTransactionFound
    )]
    #[case(
        ResolverError::NonDisputedTransaction,
        AccountTransactorError::IncompatibleTransaction
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
        let backcharger = MockBackcharger::new();
        resolver.expect(&mut account.clone(), transaction_id);
        resolver.to_return(Err(disputer_error));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );

        assert_eq!(
            processor.transact(&mut account, resolve(0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_backcharger_for_chargeback() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        backcharger.expect(&mut account, transaction_id);
        backcharger.to_return(Ok(SuccessStatus::Transacted));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );
        processor.transact(&mut account, chargeback(0)).unwrap();
    }

    #[rstest]
    #[case(BackchargerError::AccountLocked, AccountTransactorError::AccountLocked)]
    #[case(
        BackchargerError::NoTransactionFound,
        AccountTransactorError::NoTransactionFound
    )]
    #[case(
        BackchargerError::NonDisputedTransaction,
        AccountTransactorError::IncompatibleTransaction
    )]
    fn error_returned_from_backcharger_is_propagated(
        #[case] disputer_error: BackchargerError,
        #[case] expected_error: AccountTransactorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        let resolver = MockResolver::new();
        let backcharger = MockBackcharger::new();
        backcharger.expect(&mut account.clone(), transaction_id);
        backcharger.to_return(Err(disputer_error));
        let processor = SimpleAccountTransactor::new_for_test(
            depositor,
            withdrawer,
            disputer,
            resolver,
            backcharger,
        );

        assert_eq!(
            processor.transact(&mut account, chargeback(0)),
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

    fn deposit(transaction_id: TransactionId, amount: i64) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind: TransactionKind::Deposit {
                amount: Amount4DecimalBased(amount),
            },
        }
    }

    fn withdrawal(transaction_id: TransactionId, amount: i64) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind: TransactionKind::Withdrawal {
                amount: Amount4DecimalBased(amount),
            },
        }
    }

    fn dispute(transaction_id: TransactionId) -> Transaction {
        transaction(transaction_id, TransactionKind::Dispute)
    }

    fn resolve(transaction_id: TransactionId) -> Transaction {
        transaction(transaction_id, TransactionKind::Resolve)
    }

    fn chargeback(transaction_id: TransactionId) -> Transaction {
        transaction(transaction_id, TransactionKind::ChargeBack)
    }

    fn transaction(transaction_id: TransactionId, kind: TransactionKind) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind,
        }
    }
}
