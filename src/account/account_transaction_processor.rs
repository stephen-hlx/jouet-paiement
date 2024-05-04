use thiserror::Error;

use crate::{
    account::Account,
    transaction_processor::{Transaction, TransactionKind},
};

use super::processor::depositor::{DepositorError, DepositorTrait};

pub(crate) trait AccountTransactionProcessorTrait {
    fn process(
        &self,
        account: &mut Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError>;
}

pub(crate) struct AccountTransactionProcessor {
    depositor: Box<dyn DepositorTrait>,
}

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
            processor::depositor::{DepositorError, DepositorTrait},
            Account, AccountSnapshot, AccountStatus, Deposit,
        },
        model::{Amount, ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    use super::{
        AccountTransactionProcessor, AccountTransactionProcessorError,
        AccountTransactionProcessorTrait,
    };

    struct MockDepositor {
        expected_request: (Account, TransactionId, Amount),
        return_val: Result<(), DepositorError>,
    }

    impl DepositorTrait for MockDepositor {
        fn deposit(
            &self,
            account: &mut Account,
            transaction_id: TransactionId,
            _amount: Amount,
        ) -> Result<(), DepositorError> {
            let (expected_account, expected_transaction_id, _expected_amount) =
                self.expected_request.clone();
            assert_eq!(*account, expected_account);
            assert_eq!(transaction_id, expected_transaction_id);
            // assert_eq!(amount, expected_account);
            self.return_val.clone()
        }
    }

    const CLIENT_ID: ClientId = 123;

    #[test]
    fn calls_depositor_for_deposit() {
        let mut account = account(AccountStatus::Active, 0, 0, vec![]);
        let transaction_id: TransactionId = 0;
        let amount: Amount = OrderedFloat(0.0);

        let depositor = MockDepositor {
            expected_request: (account.clone(), transaction_id, amount.clone()),
            return_val: Ok(()),
        };
        let processor = AccountTransactionProcessor {
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

        let depositor = MockDepositor {
            expected_request: (account.clone(), transaction_id, amount.clone()),
            return_val: Err(depositor_error),
        };
        let processor = AccountTransactionProcessor {
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
