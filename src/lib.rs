mod txt_format;

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::marker::PhantomData;

pub trait Readable<ReadError: Error> : Sized {
    #[doc(hidden)]
    fn read<R: BufRead>(reader: &mut R) -> Result<Self, ReadError>;
}

pub struct Parser<TRecord, Reader, E>
where
    TRecord: Readable<E>,
    Reader: Read,
    E: Error,
{
    reader: BufReader<Reader>,
    read_error: Option<E>,
    _marker: PhantomData<(TRecord, E)>,
}

impl<TRecord, Reader, E> Iterator for Parser<TRecord, Reader, E>
where
    TRecord: Readable<E>,
    Reader: Read,
    E: Error,
{
    type Item = Result<TRecord, E>;

    fn next(&mut self) -> Option<Self::Item> {
        match TRecord::read(&mut self.reader) {
            Ok(record) => Some(Ok(record)),
            Err(e) => {
                self.read_error = Some(e);
                None
            }
        }
    }
}

impl<TRecord, Reader, E> Parser<TRecord, Reader, E>
where
    TRecord: Readable<E>,
    Reader: Read,
    E: Error,
{
    pub fn new(reader: Reader) -> Self {
        let reader = BufReader::new(reader);

        Self {
            reader,
            read_error: None,
            _marker: PhantomData,
        }
    }
}

pub trait Writable<WriteError: Error> {
    #[doc(hidden)]
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), WriteError>;
}

pub struct Serializer<TRecord, Writer, E>
where
    TRecord: Writable<E>,
    Writer: Write,
    E: Error,
{
    writer: Writer,
    _marker: PhantomData<(TRecord, E)>,
}

impl<TRecord, Writer, E> Serializer<TRecord, Writer, E>
where
    TRecord: Writable<E>,
    Writer: Write,
    E: Error,
{
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            _marker: PhantomData,
        }
    }

    pub fn serialize(&mut self, records: &[TRecord]) -> Result<(), E> {
        for record in records {
            record.write(&mut self.writer)?;
        }

        Ok(())
    }
}