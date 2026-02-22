use std::io::{Error, ErrorKind, Read, Write};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use crate::common::{TransactionStatus, TransactionType};
use crate::{Readable, Writable};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct YPBankCsvRecord {
    #[serde(rename = "TX_ID")]
    #[serde_as(as = "DisplayFromStr")]
    id: u32,

    #[serde(rename = "TX_TYPE")]
    transaction_type: TransactionType,

    #[serde(rename = "FROM_USER_ID")]
    #[serde_as(as = "DisplayFromStr")]
    from_user_id: u32,

    #[serde(rename = "TO_USER_ID")]
    #[serde_as(as = "DisplayFromStr")]
    to_user_id: u32,

    #[serde(rename = "AMOUNT")]
    #[serde_as(as = "DisplayFromStr")]
    amount: u64,

    #[serde(rename = "TIMESTAMP")]
    #[serde_as(as = "DisplayFromStr")]
    timestamp: u64,

    #[serde(rename = "STATUS")]
    transaction_status: TransactionStatus,
    #[serde(rename = "DESCRIPTION")]
    description: String
}

impl<R: Read> Readable<R, Error> for YPBankCsvRecord {
    type Reader = csv::Reader<R>;

    fn build_reader(source: R) -> Self::Reader {
        csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(source)
    }

    fn read(reader: &mut Self::Reader) -> Result<Self, Error> {
        let mut iter = reader.deserialize();

        match iter.next() {
            Some(Ok(record)) => Ok(record),
            Some(Err(e)) => Err(Error::new(ErrorKind::InvalidData, e)),
            None => Err(Error::new(ErrorKind::UnexpectedEof, "End of CSV")),
        }
    }
}

impl Writable<Error> for YPBankCsvRecord {
    fn write_header<W: Write>(writer: &mut W) -> Result<(), Error> {
        writer.write_all(b"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n")
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},\"{}\"",
            self.id,
            self.transaction_type,
            self.from_user_id,
            self.to_user_id,
            self.amount,
            self.timestamp,
            self.transaction_status,
            self.description.replace('"', "\"\"") // экранирование кавычек внутри description для CSV формата
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::{Parser, Serializer};

    fn sample_record() -> YPBankCsvRecord {
        YPBankCsvRecord {
            id: 1001,
            transaction_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 501,
            amount: 50000,
            timestamp: 1672531200000,
            transaction_status: TransactionStatus::Success,
            description: "Initial account funding".to_string(),
        }
    }

    #[test]
    fn read_parses_single_record_correctly() {
        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Initial account funding\"
";
        let cursor = Cursor::new(csv_data);
        let mut parser = Parser::<YPBankCsvRecord, _, _>::new(cursor);

        let record = parser.next()
            .expect("Should have a record")
            .expect("Should parse successfully");

        assert_eq!(record, sample_record());
        assert!(parser.next().is_none());
    }

    #[test]
    fn read_parses_multiple_records() {
        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,10,100,1000,SUCCESS,\"Desc 1\"
2,WITHDRAWAL,10,0,50,2000,PENDING,\"Desc 2\"
";
        let cursor = Cursor::new(csv_data);
        let mut parser = Parser::<YPBankCsvRecord, _, _>::new(cursor);

        let r1 = parser.next().unwrap().unwrap();
        assert_eq!(r1.id, 1);
        assert_eq!(r1.transaction_type, TransactionType::Deposit);
        assert_eq!(r1.from_user_id, 0);
        assert_eq!(r1.to_user_id, 10);
        assert_eq!(r1.amount, 100);
        assert_eq!(r1.timestamp, 1000);
        assert_eq!(r1.transaction_status, TransactionStatus::Success);
        assert_eq!(r1.description, "Desc 1");

        let r2 = parser.next().unwrap().unwrap();
        assert_eq!(r2.id, 2);
        assert_eq!(r2.transaction_type, TransactionType::Withdrawal);
        assert_eq!(r2.from_user_id, 10);
        assert_eq!(r2.to_user_id, 0);
        assert_eq!(r2.amount, 50);
        assert_eq!(r2.timestamp, 2000);
        assert_eq!(r2.transaction_status, TransactionStatus::Pending);
        assert_eq!(r2.description, "Desc 2");

        assert!(parser.next().is_none());
    }

    #[test]
    fn read_returns_error_on_invalid_data() {
        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
NOT_A_NUMBER,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Bad ID\"
";
        let cursor = Cursor::new(csv_data);
        let mut parser = Parser::<YPBankCsvRecord, _, _>::new(cursor);

        // Parser returns None on error, and stores error in read_error
        assert!(parser.next().is_none());

        let err = parser.read_error.expect("Should have read_error");
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn write_formats_record_correctly() {
        let mut record1 = sample_record();
        // Add a comma to force quoting by csv crate
        record1.description = "Initial account, funding".to_string();

        let record2 = YPBankCsvRecord {
            id: 1002,
            transaction_type: TransactionType::Withdrawal,
            from_user_id: 501,
            to_user_id: 0,
            amount: 100,
            timestamp: 1672531300000,
            transaction_status: TransactionStatus::Pending,
            description: "Payment".to_string(),
        };

        let writer = Cursor::new(Vec::<u8>::new());
        let mut serializer = Serializer::new(writer);

        serializer.serialize(&[record1, record2]).unwrap();

        let bytes = serializer.into_inner().into_inner();
        let output = String::from_utf8(bytes).unwrap();
        let expected = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,\"Initial account, funding\"
1002,WITHDRAWAL,501,0,100,1672531300000,PENDING,\"Payment\"
";
        assert_eq!(output, expected);
    }
}
