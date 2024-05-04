use thiserror::Error;

use crate::{
    account::Account,
    transaction_processor::{Transaction, TransactionKind},
};

use super::processor::depositor::{Depositor, DepositorError, SimpleDepositor};

pub trait AccountTransactionProcessor {
    fn process(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError>;
}

pub struct SimpleAccountTransactionProcessor {
    depositor: Box<dyn Depositor>,
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
            TransactionKind::Withdrawal { amount } => todo!(),
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

        Self {
            depositor: Box::new(depositor),
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
}

impl From<DepositorError> for AccountTransactionProcessorError {
    fn from(err: DepositorError) -> Self {
        match err {
            DepositorError::AccountLocked => Self::CannotDepositToLockedAccount,
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
            processor::depositor::{mock::MockDepositor, DepositorError},
            Account, AccountSnapshot, AccountStatus, Deposit,
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
        let mut account = account(AccountStatus::Active, 0, 0, vec![]);
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        depositor.expect(&mut account, transaction_id, amount);
        depositor.to_return(Ok(()));
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
        };
        processor.process(&mut account, deposit(0, 0)).unwrap();
    }

    // TODO: automate this for new error types
    #[rstest]
    #[case(
        DepositorError::AccountLocked,
        AccountTransactionProcessorError::CannotDepositToLockedAccount
    )]
    fn error_returned_from_depositor_is_propagated(
        #[case] depositor_error: DepositorError,
        #[case] expected_error: AccountTransactionProcessorError,
    ) {
        let mut account = account(AccountStatus::Active, 0, 0, vec![]);
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor::new();
        depositor.expect(&mut account.clone(), transaction_id, amount);
        depositor.to_return(Err(depositor_error));
        let processor = SimpleAccountTransactionProcessor {
            depositor: Box::new(depositor),
        };

        assert_eq!(
            processor.process(&mut account, deposit(0, 0)),
            Err(expected_error)
        );
    }

    fn account(
        status: AccountStatus,
        available: i32,
        held: u32,
        deposits: Vec<(TransactionId, Deposit)>,
    ) -> Account {
        Account {
            client_id: 1234,
            status,
            account_snapshot: AccountSnapshot::new(available, held),
            deposits: deposits.into_iter().collect(),
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
}
