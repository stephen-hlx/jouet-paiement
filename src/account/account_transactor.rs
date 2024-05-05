use thiserror::Error;

use crate::{
    account::Account,
    transaction_processor::{Transaction, TransactionKind},
};

use super::transactors::{
    depositor::{Depositor, DepositorError, SimpleDepositor},
    disputer::{CreditDebitDisputer, Disputer, DisputerError},
    withdrawer::{SimpleWithdrawer, Withdrawer, WithdrawerError},
};

pub trait AccountTransactor {
    fn transact(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError>;
}

pub struct SimpleAccountTransactor {
    depositor: Box<dyn Depositor + Send + Sync>,
    withdrawer: Box<dyn Withdrawer + Send + Sync>,
    disputer: Box<dyn Disputer + Send + Sync>,
}

impl AccountTransactor for SimpleAccountTransactor {
    fn transact(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError> {
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
            TransactionKind::Resolve => todo!(),
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

        Self {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        }
    }
}

#[derive(Debug, Error, PartialEq, Clone)]
pub enum AccountTransactionProcessorError {
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
}

impl From<DepositorError> for AccountTransactionProcessorError {
    fn from(err: DepositorError) -> Self {
        match err {
            DepositorError::AccountLocked => Self::CannotDepositToLockedAccount,
        }
    }
}

impl From<WithdrawerError> for AccountTransactionProcessorError {
    fn from(err: WithdrawerError) -> Self {
        match err {
            WithdrawerError::AccountLocked => Self::CannotWithdrawFromLockedAccount,
            WithdrawerError::InsufficientFund => Self::InsufficientFundForWithdrawal,
        }
    }
}

impl From<DisputerError> for AccountTransactionProcessorError {
    fn from(err: DisputerError) -> Self {
        match err {
            DisputerError::AccountLocked => Self::CannotDisputeAgainstLockedAccount,
            DisputerError::NoTransactionFound => Self::NoTransactionFound,
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
                withdrawer::{mock::MockWithdrawer, WithdrawerError},
            },
            Account, AccountSnapshot, AccountStatus,
        },
        model::{Amount, ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    use super::{AccountTransactionProcessorError, AccountTransactor, SimpleAccountTransactor};

    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        depositor.expect(&mut account, transaction_id, amount);
        depositor.to_return(Ok(()));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };
        processor.transact(&mut account, deposit(0, 0)).unwrap();
    }

    #[rstest]
    #[case(
        DepositorError::AccountLocked,
        AccountTransactionProcessorError::CannotDepositToLockedAccount
    )]
    fn error_returned_from_depositor_is_propagated(
        #[case] depositor_error: DepositorError,
        #[case] expected_error: AccountTransactionProcessorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        depositor.expect(&mut account.clone(), transaction_id, amount);
        depositor.to_return(Err(depositor_error));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };

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
        withdrawer.expect(&mut account, transaction_id, amount);
        withdrawer.to_return(Ok(()));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };
        processor.transact(&mut account, withdrawal(0, 0)).unwrap();
    }

    #[rstest]
    #[case(
        WithdrawerError::AccountLocked,
        AccountTransactionProcessorError::CannotWithdrawFromLockedAccount
    )]
    #[case(
        WithdrawerError::InsufficientFund,
        AccountTransactionProcessorError::InsufficientFundForWithdrawal
    )]
    fn error_returned_from_withdrawer_is_propagated(
        #[case] withdrawer_error: WithdrawerError,
        #[case] expected_error: AccountTransactionProcessorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        withdrawer.expect(&mut account.clone(), transaction_id, amount);
        withdrawer.to_return(Err(withdrawer_error));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };

        assert_eq!(
            processor.transact(&mut account, withdrawal(0, 0)),
            Err(expected_error)
        );
    }

    #[test]
    fn calls_disputer_for_withdrawal() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        disputer.expect(&mut account, transaction_id);
        disputer.to_return(Ok(()));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };
        processor.transact(&mut account, dispute(0)).unwrap();
    }

    #[rstest]
    #[case(
        DisputerError::AccountLocked,
        AccountTransactionProcessorError::CannotDisputeAgainstLockedAccount
    )]
    #[case(
        DisputerError::NoTransactionFound,
        AccountTransactionProcessorError::NoTransactionFound
    )]
    fn error_returned_from_disputer_is_propagated(
        #[case] disputer_error: DisputerError,
        #[case] expected_error: AccountTransactionProcessorError,
    ) {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;

        let depositor = MockDepositor::new();
        let withdrawer = MockWithdrawer::new();
        let disputer = MockDisputer::new();
        disputer.expect(&mut account.clone(), transaction_id);
        disputer.to_return(Err(disputer_error));
        let processor = SimpleAccountTransactor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
            disputer: Box::new(disputer),
        };

        assert_eq!(
            processor.transact(&mut account, dispute(0)),
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
        Transaction {
            client_id: CLIENT_ID,
            transaction_id,
            kind: TransactionKind::Dispute,
        }
    }
}
