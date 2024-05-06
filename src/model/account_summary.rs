use csv::WriterBuilder;
use thiserror::Error;

use crate::account::{Account, AccountSnapshot, AccountStatus};

use super::{AccountSummary, Amount4DecimalBased};

impl From<Account> for AccountSummary {
    fn from(account: Account) -> Self {
        AccountSummary::from(&account)
    }
}

impl From<&Account> for AccountSummary {
    fn from(account: &Account) -> Self {
        let AccountSnapshot { available, held } = account.account_snapshot;
        let total = Amount4DecimalBased(available.0 + held.0);
        Self {
            client_id: account.client_id,
            available: available.to_str(),
            held: held.to_str(),
            total: total.to_str(),
            locked: account.status == AccountStatus::Locked,
        }
    }
}

pub struct AccountSummaryWriter;

#[derive(Debug, Error)]
pub enum AccountSummaryWriterError {
    #[error("Failed to serialise the AccountSummary: {0}")]
    SerialisationError(String),
}

impl AccountSummaryWriter {
    pub fn write(accounts: Vec<Account>) -> Result<Vec<u8>, AccountSummaryWriterError> {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        for summary in accounts
            .into_iter()
            .map(|account| AccountSummary::from(account))
        {
            wtr.serialize(summary).unwrap();
        }
        match wtr.into_inner() {
            Ok(chars) => Ok(chars),
            Err(e) => return Err(AccountSummaryWriterError::SerialisationError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::account::{
        Account, AccountSnapshot,
        AccountStatus::{Active, Locked},
    };

    use super::AccountSummaryWriter;

    #[test]
    fn can_write_account_summary_data_as_csv() {
        let account_1 = Account::new(
            1122,
            Active,
            AccountSnapshot::new(111, 222),
            HashMap::new(),
            HashMap::new(),
        );
        let account_2 = Account::new(
            3344,
            Locked,
            AccountSnapshot::new(3_330_000, 4_440_000),
            HashMap::new(),
            HashMap::new(),
        );

        assert_eq!(
            String::from_utf8(AccountSummaryWriter::write(vec![account_1, account_2]).unwrap())
                .unwrap(),
            "\
            client,available,held,total,locked\n\
            1122,0.0111,0.0222,0.0333,false\n\
            3344,333.0000,444.0000,777.0000,true\n"
        );
    }
}
