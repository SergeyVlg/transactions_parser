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
use crate::common::Transaction;

pub trait Readable<Source: Read> : Sized + Into<Transaction> {
    type Reader;
    type Error: Error + IsEofError + From<std::io::Error>;

    #[doc(hidden)]
    fn build_reader(source: Source) -> Self::Reader;
    #[doc(hidden)]
    fn read(reader: &mut Self::Reader) -> Result<Self, Self::Error>;
}

trait IsEofError {
    fn is_eof(&self) -> bool;
}

pub struct Parser<TRecord, Source>
where
    TRecord: Readable<Source>,
    Source: Read
{
    reader: TRecord::Reader,
    pub read_error: Option<TRecord::Error>,
    _marker: PhantomData<Source>,
}

impl<TRecord, Source> Iterator for Parser<TRecord, Source>
where
    TRecord: Readable<Source>,
    Source: Read,
{
    //type Item = Result<TRecord, TRecord::Error>;
    type Item = TRecord;

    // может сделать Item обычным TRecord, а ошибку сохранять в поле read_error?
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
    pub fn new(source: Source) -> Self {
        let reader = TRecord::build_reader(source);

        Self {
            reader,
            read_error: None,
            _marker: PhantomData,
        }
    }
}

pub trait Writable: From<Transaction> {
    type Error: Error + From<std::io::Error>;

    #[doc(hidden)]
    fn write_header<W: Write>(writer: &mut W) -> Result<(), Self::Error>;

    #[doc(hidden)]
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Self::Error>;
}

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
    pub fn new(target: Target) -> Self {
        let buffered_target = BufWriter::new(target);
        Self {
            target: buffered_target,
            _marker: PhantomData,
        }
    }

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