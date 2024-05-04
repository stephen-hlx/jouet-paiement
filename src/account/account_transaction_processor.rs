use thiserror::Error;

use crate::{
    account::Account,
    transaction_processor::{Transaction, TransactionKind},
};

use super::processor::{
    depositor::{Depositor, DepositorError, SimpleDepositor},
    withdrawer::{SimpleWithdrawer, Withdrawer, WithdrawerError},
};

pub trait AccountTransactionProcessor {
    fn process(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError>;
}

pub struct SimpleAccountTransactionProcessor {
    depositor: Box<dyn Depositor + Send + Sync>,
    withdrawer: Box<dyn Withdrawer + Send + Sync>,
}

impl AccountTransactionProcessor for SimpleAccountTransactionProcessor {
    fn process(
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
            TransactionKind::Dispute => todo!(),
            TransactionKind::Resolve => todo!(),
            TransactionKind::ChargeBack => todo!(),
        }
        Ok(())
    }
}

impl SimpleAccountTransactionProcessor {
    pub fn new() -> Self {
        let depositor = SimpleDepositor;
        let withdrawer = SimpleWithdrawer;

        Self {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        account::{
            processor::{
                depositor::{mock::MockDepositor, DepositorError},
                withdrawer::{mock::MockWithdrawer, WithdrawerError},
            },
            Account, AccountSnapshot, AccountStatus,
        },
        model::{Amount, ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    use super::{
        AccountTransactionProcessor, AccountTransactionProcessorError,
        SimpleAccountTransactionProcessor,
    };

    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut account = some_account();
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        depositor.expect(&mut account, transaction_id, amount);
        depositor.to_return(Ok(()));
        let withdrawer = MockWithdrawer::new();
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
        };
        processor.process(&mut account, deposit(0, 0)).unwrap();
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
        depositor.expect(&mut account.clone(), transaction_id, amount);
        depositor.to_return(Err(depositor_error));
        let withdrawer = MockWithdrawer::new();
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
        };

        assert_eq!(
            processor.process(&mut account, deposit(0, 0)),
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
        withdrawer.expect(&mut account, transaction_id, amount);
        withdrawer.to_return(Ok(()));
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
        };
        processor.process(&mut account, withdrawal(0, 0)).unwrap();
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
        withdrawer.expect(&mut account.clone(), transaction_id, amount);
        withdrawer.to_return(Err(withdrawer_error));
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
            withdrawer: Box::new(withdrawer),
        };

        assert_eq!(
            processor.process(&mut account, withdrawal(0, 0)),
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
}
