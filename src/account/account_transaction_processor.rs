#[cfg(test)]
use mockall::automock;
use mockall_double::double;
use thiserror::Error;

use crate::{
    account::Account,
    transaction_processor::{Transaction, TransactionKind},
};

#[double]
use super::processor::depositor::Depositor;

use super::processor::depositor::DepositorError;

pub(crate) struct AccountTransactionProcessor {
    depositor: Depositor,
}

#[cfg_attr(test, automock)]
impl AccountTransactionProcessor {
    pub(crate) fn process(
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

#[derive(Debug, Error, PartialEq, Clone)]
pub(crate) enum AccountTransactionProcessorError {
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
            processor::depositor::{DepositorError, MockDepositor},
            Account, AccountSnapshot, AccountStatus, Deposit,
        },
        model::{ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    use super::{AccountTransactionProcessor, AccountTransactionProcessorError};

    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut depositor = MockDepositor::new();
        let mut account = account(AccountStatus::Active, 0, 0, vec![]);
        depositor
            .expect_deposit()
            .times(1)
            // a little sloppy here since OrderedFloat does not work well with predicate::eq
            // .with(
            //     predicate::eq(account),
            //     predicate::eq(transaction_id),
            //     predicate::eq(amount),
            // )
            .return_const(Ok(()));
        let processor = AccountTransactionProcessor { depositor };
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
        let mut depositor = MockDepositor::new();
        depositor
            .expect_deposit()
            .return_const(Err(depositor_error));
        let processor = AccountTransactionProcessor { depositor };

        let mut account = account(AccountStatus::Active, 0, 0, vec![]);
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
