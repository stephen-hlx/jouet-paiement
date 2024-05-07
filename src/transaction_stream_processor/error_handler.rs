use crate::{
    account::account_transactor::AccountTransactorError::{
        AccountLocked, IncompatibleTransaction, InsufficientFundForWithdrawal, NoTransactionFound,
    },
    transaction_processor::TransactionProcessorError,
};

use super::ErrorHandler;

pub(crate) struct SimpleErrorHandler;

impl ErrorHandler for SimpleErrorHandler {
    fn handle(
        &self,
        transaction_processor_error: TransactionProcessorError,
    ) -> Result<(), TransactionProcessorError> {
        match transaction_processor_error {
            TransactionProcessorError::AccountTransactionError(
                ref _transaction,
                ref account_transactor_error,
            ) => match account_transactor_error {
                AccountLocked => Err(transaction_processor_error),
                IncompatibleTransaction => Err(transaction_processor_error),
                InsufficientFundForWithdrawal => Ok(()),
                NoTransactionFound => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{
        account::account_transactor::AccountTransactorError::{
            self, AccountLocked, IncompatibleTransaction, InsufficientFundForWithdrawal,
            NoTransactionFound,
        },
        model::{Amount4DecimalBased, Transaction},
        transaction_processor::TransactionProcessorError,
        transaction_stream_processor::ErrorHandler,
    };

    use super::SimpleErrorHandler;

    #[rstest]
    #[case(account_lock(), Err(account_lock()))]
    #[case(incompatible(), Err(incompatible()))]
    #[case(insufficient_fund(),    Ok(()))]
    #[case(no_transaction_found(), Ok(()))]
    fn simple_error_handler_works(
        #[case] error: TransactionProcessorError,
        #[case] after_handling: Result<(), TransactionProcessorError>,
    ) {
        let handler = SimpleErrorHandler;
        assert_eq!(handler.handle(error), after_handling);
    }

    fn account_lock() -> TransactionProcessorError {
        transaction_processor_error(AccountLocked)
    }

    fn incompatible() -> TransactionProcessorError {
        transaction_processor_error(IncompatibleTransaction)
    }

    fn insufficient_fund() -> TransactionProcessorError {
        transaction_processor_error(InsufficientFundForWithdrawal)
    }

    fn no_transaction_found() -> TransactionProcessorError {
        transaction_processor_error(NoTransactionFound)
    }

    fn transaction_processor_error(
        account_transactor_error: AccountTransactorError,
    ) -> TransactionProcessorError {
        TransactionProcessorError::AccountTransactionError(
            Transaction {
                client_id: 123,
                transaction_id: 456,
                kind: crate::model::TransactionKind::Deposit {
                    amount: Amount4DecimalBased(1),
                },
            },
            account_transactor_error,
        )
    }
}
