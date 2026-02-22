use std::io::{BufReader, Error, ErrorKind, Read, Write};
use crate::common::{TransactionStatus, TransactionType};
use crate::{Readable, Writable};

#[derive(Debug, PartialEq)]
struct YPBankBinRecord {
    id: u64,
    transaction_type: TransactionType,
    from_user_id: u64,
    to_user_id: u64,
    amount: i64,
    timestamp: u64,
    transaction_status: TransactionStatus,
    description: String
}

impl<R: Read> Readable<R, Error> for YPBankBinRecord {
    type Reader = BufReader<R>;

    fn build_reader(source: R) -> Self::Reader {
        BufReader::new(source)
    }

    fn read(reader: &mut Self::Reader) -> Result<Self, Error> {
        let mut magic = [0u8; 4];

        let bytes_read = reader.read(&mut magic)?;

        if bytes_read == 0 {
            //normal eof
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        if bytes_read < 4 {
            return Err(Error::new(ErrorKind::InvalidData, "File truncated inside magic"));
        }

        if &magic != b"YPBN" {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid magic bytes"));
        }

        let mut size_buf = [0u8; 4];
        reader.read_exact(&mut size_buf)?;
        let _record_size = u32::from_be_bytes(size_buf);

        let mut u64_buf = [0u8; 8];
        reader.read_exact(&mut u64_buf)?;
        let id = u64::from_be_bytes(u64_buf);

        let mut u8_buf = [0u8; 1];
        reader.read_exact(&mut u8_buf)?;
        let transaction_type = TransactionType::from(u8::from_be_bytes(u8_buf));

        reader.read_exact(&mut u64_buf)?;
        let from_user_id = u64::from_be_bytes(u64_buf);

        reader.read_exact(&mut u64_buf)?;
        let to_user_id = u64::from_be_bytes(u64_buf);

        reader.read_exact(&mut u64_buf)?;
        let amount = i64::from_be_bytes(u64_buf);

        reader.read_exact(&mut u64_buf)?;
        let timestamp = u64::from_be_bytes(u64_buf);

        reader.read_exact(&mut u8_buf)?;
        let transaction_status = TransactionStatus::from(u8::from_be_bytes(u8_buf));

        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let desc_len = u32::from_be_bytes(len_buf) as usize;
        let mut description = String::new();

        if desc_len > 0 {
            let mut desc_bytes = vec![0u8; desc_len];
            reader.read_exact(&mut desc_bytes)?;
            description = String::from_utf8(desc_bytes)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        }

        Ok(YPBankBinRecord {
            id,
            transaction_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp,
            transaction_status,
            description,
        })
    }
}

impl Writable<Error> for YPBankBinRecord {
    fn write_header<W: Write>(_: &mut W) -> Result<(), Error> {
        Ok(())
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_all(b"YPBN")?;

        let record_size = 8 + 1 + 8 + 8 + 8 + 8 + 1 + 4 + self.description.len() as u32;
        writer.write_all(&record_size.to_be_bytes())?;

        writer.write_all(&self.id.to_be_bytes())?;
        writer.write_all(&[self.transaction_type.into()])?;
        writer.write_all(&self.from_user_id.to_be_bytes())?;
        writer.write_all(&self.to_user_id.to_be_bytes())?;
        writer.write_all(&self.amount.to_be_bytes())?;
        writer.write_all(&self.timestamp.to_be_bytes())?;
        writer.write_all(&[self.transaction_status.into()])?;

        let desc_bytes = self.description.as_bytes();
        let desc_len = desc_bytes.len() as u32;
        writer.write_all(&desc_len.to_be_bytes())?;

        if desc_len > 0 {
            writer.write_all(desc_bytes)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, Serializer};
    use std::io::Cursor;

    fn sample_record() -> YPBankBinRecord {
        YPBankBinRecord {
            id: 1234567890,
            transaction_type: TransactionType::Transfer,
            from_user_id: 111,
            to_user_id: 222,
            amount: 1000,
            timestamp: 1633056800000,
            transaction_status: TransactionStatus::Failure,
            description: "Test transaction".to_string(),
        }
    }

    #[test]
    fn write_serializes_correctly() {
        let record = sample_record();
        let writer = Cursor::new(Vec::new());
        let mut serializer = Serializer::new(writer);

        serializer.serialize(&[sample_record()]).unwrap();

        let buffer = serializer.into_inner().into_inner();

        assert_eq!(&buffer[0..4], b"YPBN");

        // id(8) + type(1) + from(8) + to(8) + amount(8) + timestamp(8) + status(1) = 42 bytes
        // + desc_len(4) + description("Test transaction".len() = 16) = 20 bytes
        // Total = 62 bytes
        let size_bytes = &buffer[4..8];
        let size = u32::from_be_bytes(size_bytes.try_into().unwrap());
        assert_eq!(size, 62);

        let body = &buffer[8..];

        assert_eq!(&body[0..8], &record.id.to_be_bytes());
        assert_eq!(body[8], record.transaction_type as u8);
        assert_eq!(&body[9..17], &record.from_user_id.to_be_bytes());

        assert_eq!(&body[17..25], &record.to_user_id.to_be_bytes());
        assert_eq!(&body[25..33], &record.amount.to_be_bytes());
        assert_eq!(&body[33..41], &record.timestamp.to_be_bytes());

        assert_eq!(body[41], record.transaction_status.into());
        assert_eq!(&body[42..46], &(record.description.len() as u32).to_be_bytes());
        assert_eq!(&body[46..], record.description.as_bytes());
    }

    #[test]
    fn read_deserializes_correctly() {
        let original_record = sample_record();
        let mut buffer = Vec::new();

        original_record.write(&mut buffer).unwrap();

        let cursor = Cursor::new(buffer);
        let mut parser = Parser::<YPBankBinRecord, _, _>::new(cursor);

        let read_record = parser.next().expect("Should return a record").expect("Should be Ok");

        assert_eq!(read_record, original_record);
        assert!(parser.next().is_none());
        assert!(parser.read_error.is_none(), "Expected no read error, got: {:?}", parser.read_error);
    }

    #[test]
    fn read_fails_on_invalid_magic() {
        let record = sample_record();
        let mut buffer = Vec::new();
        record.write(&mut buffer).unwrap();

        buffer[0] = b'X'; //break magic bytes

        let cursor = Cursor::new(buffer);
        let mut parser = Parser::<YPBankBinRecord, _, _>::new(cursor);

        let result = parser.next();
        assert!(result.is_none());

        if let Some(err) = parser.read_error {
            assert_eq!(err.kind(), ErrorKind::InvalidData);
        } else {
            panic!("Expected an error, but got none");
        }
    }

    #[test]
    fn read_fails_on_unexpected_eof() {
        let record = sample_record();
        let mut buffer = Vec::new();
        record.write(&mut buffer).unwrap();

        let truncated_len = buffer.len() - 5;
        let truncated_buffer = &buffer[..truncated_len];

        let cursor = Cursor::new(truncated_buffer);
        let mut parser = Parser::<YPBankBinRecord, _, _>::new(cursor);

        let result = parser.next();
        assert!(result.is_none());
        assert!(parser.read_error.is_none(), "Expected no read error, got: {:?}", parser.read_error);
    }

    #[test]
    fn write_serializes_two_records_correctly() {
        let record1 = sample_record();
        let record2 = YPBankBinRecord {
            id: 999999,
            transaction_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 555,
            amount: 500,
            timestamp: 1700000000,
            transaction_status: TransactionStatus::Success,
            description: "Second tx".to_string(),
        };

        let writer = Cursor::new(Vec::new());
        let mut serializer = Serializer::new(writer);
        let record2_id = record2.id;

        serializer.serialize(&[sample_record(), record2]).unwrap();

        let buffer = serializer.into_inner().into_inner();
        assert_eq!(&buffer[0..4], b"YPBN");

        let size1 = u32::from_be_bytes(buffer[4..8].try_into().unwrap());
        assert_eq!(size1, 62);

        let offset2 = 8 + size1 as usize;

        //check second record starts with magic bytes
        assert_eq!(&buffer[offset2..offset2+4], b"YPBN");

        let size2_bytes = &buffer[offset2+4..offset2+8];
        let size2 = u32::from_be_bytes(size2_bytes.try_into().unwrap());
        // id(8) + type(1) + from(8) + to(8) + amount(8) + timestamp(8) + status(1) + desc_len(4) = 46 bytes
        // + description("Second tx".len() = 9) = 55 bytes
        assert_eq!(size2, 55);

        let body2 = &buffer[offset2+8..];
        assert_eq!(&body2[0..8], &record2_id.to_be_bytes());
        assert_eq!(&body2[46..], b"Second tx");
    }

    #[test]
    fn read_deserializes_two_records_correctly() {
        let record1 = sample_record();
        let record2 = YPBankBinRecord {
            id: 888,
            transaction_type: TransactionType::Withdrawal,
            from_user_id: 333,
            to_user_id: 0,
            amount: 200,
            timestamp: 1600000000,
            transaction_status: TransactionStatus::Pending,
            description: "Another one".to_string(),
        };

        let mut buffer = Vec::new();

        record1.write(&mut buffer).unwrap();
        record2.write(&mut buffer).unwrap();

        let cursor = Cursor::new(buffer);
        let mut parser = Parser::<YPBankBinRecord, _, _>::new(cursor);

        let read_r1 = parser.next().expect("First record").expect("Ok");
        assert_eq!(read_r1, record1);

        let read_r2 = parser.next().expect("Second record").expect("Ok");
        assert_eq!(read_r2, record2);

        assert!(parser.next().is_none());
        assert!(parser.read_error.is_none(), "Expected no read error, got: {:?}", parser.read_error);
    }
}
