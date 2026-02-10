pub mod txt_format {
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::io::{BufRead, BufReader, Read, Write};
    use crate::Parser;

    enum TransactionType {
        Deposit,
        Transfer,
        Withdrawal
    }

    enum TransactionStatus {
        Pending,
        Success,
        Failure
    }

    struct YPBankTextRecord {
        id: u32,
        transaction_type: TransactionType,
        from_user_id: u32,
        to_user_id: u32,
        amount: u64,
        timestamp: u64,
        transaction_status: TransactionStatus,
        description: String
    }

    #[derive(Debug)]
    enum TextRecordError {
        WrongLineFormat { line_index: u32 },
        MissingColonAfterKey { line_index: u32 },
        UnexpectedKey { line_index: u32 },
        ReadLineError(std::io::Error),
    }

    impl Display for TextRecordError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TextRecordError {}

    impl Parser<TextRecordError> for YPBankTextRecord {
        fn from_read<R: Read>(reader: &mut R) -> Result<Self, TextRecordError> {
            let mut buff_reader = BufReader::new(reader);
            let mut line_index: u32 = 0;

            for line in buff_reader.lines() {
                if let Ok(line) = line {
                    if line.trim().starts_with('#') {
                        continue;
                    }

                    let line_values: Vec<&str> = line.trim().split(' ').collect();

                    if line_values.len() <= 1 {
                        return Err(TextRecordError::WrongLineFormat { line_index })
                    }

                    let key = line_values[0];
                    let Some(':') = key.chars().nth(key.chars().count() - 1) else {
                        return Err(TextRecordError::MissingColonAfterKey { line_index })
                    };
                    let key = key.trim_matches(':');

                    match key {
                        "TX_ID" => line_index = line_index + 1,
                        _ => return Err(TextRecordError::UnexpectedKey { line_index })
                    }

                } else {
                    return Err(TextRecordError::ReadLineError(line.unwrap_err()));
                }

                line_index += 1;
            }
            Ok(());
        }

        fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), &(dyn Error + 'static)> {
            todo!()
        }
    }
}