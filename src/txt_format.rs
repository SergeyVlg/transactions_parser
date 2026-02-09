pub mod txt_format {
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::io::{Read, Write};
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
        SomeError,
    }

    impl Display for TextRecordError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TextRecordError {}

    impl Parser<TextRecordError> for YPBankTextRecord {
        fn from_read<R: Read>(reader: &mut R) -> Result<Self, TextRecordError> {
            Err(TextRecordError::SomeError)
        }

        fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), &(dyn Error + 'static)> {
            todo!()
        }
    }
}