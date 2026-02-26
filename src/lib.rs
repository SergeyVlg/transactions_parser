mod common;
mod txt_format;
mod csv_format;
mod bin_format;

use std::error::Error;
use std::io::{BufWriter, Read, Write};
use std::marker::PhantomData;

pub use txt_format::{YPBankTextRecord};
pub use csv_format::{YPBankCsvRecord};
pub use bin_format::{YPBankBinRecord};
pub use common::{Transaction, TransactionType, TransactionStatus};

/// Трейт для типов, поддерживающих чтение из источника данных.
///
/// Этот трейт позволяет абстрагироваться от конкретного формата данных (CSV, бинарный и т.д.)
/// и способа их чтения. Типы, реализующие этот трейт, могут быть преобразованы в `Transaction`.
pub trait Readable<Source: Read> : Sized + Into<Transaction> {
    /// Тип читателя, используемого для извлечения данных.
    type Reader;
    /// Тип ошибки, возникающей при чтении.
    type Error: Error + IsEofError + From<std::io::Error> + Into<std::io::Error>;

    #[doc(hidden)]
    fn build_reader(source: Source) -> Self::Reader;
    #[doc(hidden)]
    fn read(reader: &mut Self::Reader) -> Result<Self, Self::Error>;
}

/// Трейт для проверки, является ли ошибка указанием на конец файла (EOF).
pub trait IsEofError {
    /// Возвращает `true`, если ошибка соответствует концу файла.
    fn is_eof(&self) -> bool;
}

/// Парсер, преобразующий поток байтов в поток записей определенного типа.
///
/// `Parser` читает исходный поток (`Source`) и использует реализацию `Readable`
/// для `TRecord`, чтобы итеративно извлекать записи.
pub struct Parser<TRecord, Source>
where
    TRecord: Readable<Source>,
    Source: Read
{
    reader: TRecord::Reader,
    /// Содержит ошибку чтения, если она произошла в процессе итерации.
    /// После возникновения ошибки итератор будет возвращать `None`.
    pub read_error: Option<TRecord::Error>,
    _marker: PhantomData<Source>,
}

impl<TRecord, Source> Iterator for Parser<TRecord, Source>
where
    TRecord: Readable<Source>,
    Source: Read,
{
    type Item = TRecord;

    fn next(&mut self) -> Option<Self::Item> {
        match TRecord::read(&mut self.reader) {
            Ok(record) => Some(record),
            Err(e) if e.is_eof() => None,
            Err(e) => {
                self.read_error = Some(e);
                None
            }
        }
    }
}

impl<TRecord, Source> Parser<TRecord, Source>
where
    TRecord: Readable<Source>,
    Source: Read
{
    /// Создает новый экземпляр парсера из источника данных.
    pub fn new(source: Source) -> Self {
        let reader = TRecord::build_reader(source);

        Self {
            reader,
            read_error: None,
            _marker: PhantomData,
        }
    }
}

/// Трейт для типов, поддерживающих запись в поток данных.
///
/// Позволяет сериализовать данные транзакции в конкретный формат.
pub trait Writable: From<Transaction> {
    /// Тип ошибки, возникающей при записи.
    type Error: Error + From<std::io::Error> + Into<std::io::Error>;

    #[doc(hidden)]
    fn write_header<W: Write>(writer: &mut W) -> Result<(), Self::Error>;

    #[doc(hidden)]
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Self::Error>;
}

/// Сериализатор, преобразующий поток записей в байты и записывающий их в целевой поток.
pub struct Serializer<TRecord, Target>
where
    TRecord: Writable,
    Target: Write,
{
    target: BufWriter<Target>,
    _marker: PhantomData<TRecord>,
}

impl<TRecord, Target> Serializer<TRecord, Target>
where
    TRecord: Writable,
    Target: Write
{
    /// Создает новый сериализатор, который будет писать в указанный `target`.
    ///
    /// `target` автоматически оборачивается в `BufWriter` для эффективности.
    pub fn new(target: Target) -> Self {
        let buffered_target = BufWriter::new(target);
        Self {
            target: buffered_target,
            _marker: PhantomData,
        }
    }

    /// Сериализует коллекцию записей и записывает их в целевой поток.
    ///
    /// Сначала записывается заголовок (если предусмотрен форматом), затем все записи,
    /// после чего буфер сбрасывается.
    pub fn serialize<I>(&mut self, records: I) -> Result<(), TRecord::Error>
    where I : IntoIterator<Item = TRecord>,
    {
        TRecord::write_header(&mut self.target)?;

        for record in records {
            record.write(&mut self.target)?;
        }

        self.target.flush()?;

        Ok(())
    }

    #[cfg(test)]
    pub fn into_inner(self) -> BufWriter<Target> {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Read, Write};

    // --- Mock record ---

    #[derive(Debug, Clone, PartialEq)]
    struct MockRecord {
        id: u64,
    }

    impl From<MockRecord> for Transaction {
        fn from(record: MockRecord) -> Self {
            Transaction {
                id: record.id,
                transaction_type: TransactionType::Deposit,
                from_user_id: 0,
                to_user_id: 0,
                amount: 0,
                timestamp: 0,
                transaction_status: TransactionStatus::Success,
                description: String::new(),
            }
        }
    }

    impl From<Transaction> for MockRecord {
        fn from(t: Transaction) -> Self {
            MockRecord { id: t.id }
        }
    }

    // --- Mock error ---

    #[derive(Debug)]
    enum MockError {
        Eof,
        Io(io::Error),
    }

    impl std::fmt::Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MockError::Eof => write!(f, "EOF"),
                MockError::Io(e) => write!(f, "{}", e),
            }
        }
    }

    impl Error for MockError {}

    impl IsEofError for MockError {
        fn is_eof(&self) -> bool {
            matches!(self, MockError::Eof)
        }
    }

    impl From<io::Error> for MockError {
        fn from(e: io::Error) -> Self {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                MockError::Eof
            } else {
                MockError::Io(e)
            }
        }
    }

    impl From<MockError> for io::Error {
        fn from(error: MockError) -> Self {
            match error {
                MockError::Eof => io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"),
                MockError::Io(e) => e,
            }
        }
    }

    // --- Readable impl ---

    impl Readable<Cursor<Vec<u8>>> for MockRecord {
        type Reader = Cursor<Vec<u8>>;
        type Error = MockError;

        fn build_reader(source: Cursor<Vec<u8>>) -> Self::Reader {
            source
        }

        fn read(reader: &mut Self::Reader) -> Result<Self, Self::Error> {
            let mut buf = [0u8; 8];
            let n = reader.read(&mut buf)?;
            if n == 0 {
                return Err(MockError::Eof);
            }
            if n < 8 {
                return Err(MockError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "incomplete record",
                )));
            }
            Ok(MockRecord {
                id: u64::from_le_bytes(buf),
            })
        }
    }

    // --- Writable impl ---

    impl Writable for MockRecord {
        type Error = MockError;

        fn write_header<W: Write>(_writer: &mut W) -> Result<(), Self::Error> {
            Ok(())
        }

        fn write<W: Write>(&self, writer: &mut W) -> Result<(), Self::Error> {
            writer.write_all(&self.id.to_le_bytes()).map_err(MockError::Io)
        }
    }

    // --- Helper ---

    fn make_source(ids: &[u64]) -> Cursor<Vec<u8>> {
        let bytes: Vec<u8> = ids.iter().flat_map(|id| id.to_le_bytes()).collect();
        Cursor::new(bytes)
    }

    fn sample_transaction(id: u64) -> Transaction {
        Transaction {
            id,
            transaction_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 0,
            amount: 0,
            timestamp: 0,
            transaction_status: TransactionStatus::Success,
            description: String::new(),
        }
    }

    // ==================== Parser tests ====================

    #[test]
    fn parser_reads_multiple_records() {
        let source = make_source(&[1, 2, 3]);
        let parser = Parser::<MockRecord, _>::new(source);
        let records: Vec<MockRecord> = parser.collect();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].id, 1);
        assert_eq!(records[1].id, 2);
        assert_eq!(records[2].id, 3);
    }

    #[test]
    fn parser_empty_source_yields_no_records() {
        let source = make_source(&[]);
        let parser = Parser::<MockRecord, _>::new(source);
        let records: Vec<MockRecord> = parser.collect();

        assert!(records.is_empty());
    }

    #[test]
    fn parser_single_record() {
        let source = make_source(&[42]);
        let parser = Parser::<MockRecord, _>::new(source);
        let records: Vec<MockRecord> = parser.collect();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, 42);
    }

    #[test]
    fn parser_sets_read_error_on_invalid_data() {
        // 3 bytes — incomplete u64, not EOF and not a valid record
        let source = Cursor::new(vec![1u8, 2, 3]);
        let mut parser = Parser::<MockRecord, _>::new(source);
        let first = parser.next();

        // Should get None (error path) and read_error should be set
        assert!(first.is_none());
        assert!(parser.read_error.is_some());
    }

    #[test]
    fn parser_stops_after_error() {
        // valid record followed by incomplete data
        let mut bytes = 1u64.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0xFFu8; 3]); // incomplete
        let source = Cursor::new(bytes);

        let mut parser = Parser::<MockRecord, _>::new(source);

        let first = parser.next();
        assert!(first.is_some());
        assert_eq!(first.unwrap().id, 1);

        let second = parser.next();
        assert!(second.is_none());
        assert!(parser.read_error.is_some());
    }

    #[test]
    fn parser_converts_records_to_transactions() {
        let source = make_source(&[10, 20]);
        let parser = Parser::<MockRecord, _>::new(source);
        let transactions: Vec<Transaction> = parser.map(|r| r.into()).collect();

        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].id, 10);
        assert_eq!(transactions[1].id, 20);
    }

    #[test]
    fn parser_no_error_after_successful_parse() {
        let source = make_source(&[1, 2]);
        let mut parser = Parser::<MockRecord, _>::new(source);

        while parser.next().is_some() {}

        assert!(parser.read_error.is_none());
    }

    // ==================== Serializer tests ====================

    #[test]
    fn serializer_writes_multiple_records() {
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);
        let records = vec![MockRecord { id: 1 }, MockRecord { id: 2 }, MockRecord { id: 3 }];

        serializer.serialize(records).unwrap();

        let output = serializer.into_inner().into_inner().unwrap();
        assert_eq!(output, make_source(&[1, 2, 3]).into_inner());
    }

    #[test]
    fn serializer_empty_iterator_produces_empty_output() {
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);
        let records: Vec<MockRecord> = vec![];

        serializer.serialize(records).unwrap();

        let output = serializer.into_inner().into_inner().unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn serializer_single_record() {
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);

        serializer.serialize(vec![MockRecord { id: 99 }]).unwrap();

        let output = serializer.into_inner().into_inner().unwrap();
        assert_eq!(output, 99u64.to_le_bytes().to_vec());
    }

    #[test]
    fn serializer_from_transactions() {
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);
        let records: Vec<MockRecord> = vec![5, 10]
            .into_iter()
            .map(|id| MockRecord::from(sample_transaction(id)))
            .collect();

        serializer.serialize(records).unwrap();

        let output = serializer.into_inner().into_inner().unwrap();
        let expected: Vec<u8> = [5u64, 10u64]
            .iter()
            .flat_map(|id| id.to_le_bytes())
            .collect();
        assert_eq!(output, expected);
    }

    // ==================== Round-trip test ====================

    #[test]
    fn round_trip_serialize_then_parse() {
        let original = vec![MockRecord { id: 100 }, MockRecord { id: 200 }, MockRecord { id: 300 }];

        // Serialize
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);
        serializer.serialize(original.clone()).unwrap();
        let bytes = serializer.into_inner().into_inner().unwrap();

        // Parse back
        let source = Cursor::new(bytes);
        let parser = Parser::<MockRecord, _>::new(source);
        let parsed: Vec<MockRecord> = parser.collect();

        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_via_transactions() {
        let ids = [7, 14, 21];
        let transactions: Vec<Transaction> = ids.iter().map(|&id| sample_transaction(id)).collect();

        // Transaction → MockRecord → serialize
        let records: Vec<MockRecord> = transactions.iter().cloned().map(MockRecord::from).collect();
        let target: Vec<u8> = Vec::new();
        let mut serializer = Serializer::<MockRecord, _>::new(target);
        serializer.serialize(records).unwrap();
        let bytes = serializer.into_inner().into_inner().unwrap();

        // parse → MockRecord → Transaction
        let source = Cursor::new(bytes);
        let parser = Parser::<MockRecord, _>::new(source);
        let parsed_transactions: Vec<Transaction> = parser.map(|r| r.into()).collect();

        assert_eq!(parsed_transactions.len(), transactions.len());
        for (parsed, original) in parsed_transactions.iter().zip(transactions.iter()) {
            assert_eq!(parsed.id, original.id);
        }
    }
}
