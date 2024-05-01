use crate::account::{Account, Transaction};

pub(crate) trait AccountTransactionProcessor {
    fn process(
        &self,
        account: Account,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionProcessorError>;
}

pub(crate) enum AccountTransactionProcessorError {}
