mod txt_format;
mod csv_format;
mod common;

use std::error::Error;
use std::io::{Read, Write};
use std::marker::PhantomData;

pub trait Readable<Source: Read, E: Error> : Sized {
    type Reader;

    #[doc(hidden)]
    fn build_reader(source: Source) -> Self::Reader;
    #[doc(hidden)]
    fn read(reader: &mut Self::Reader) -> Result<Self, E>;
}

pub struct Parser<TRecord, Source, E>
where
    TRecord: Readable<Source, E>,
    Source: Read,
    E: Error,
{
    reader: TRecord::Reader,
    read_error: Option<E>,
    _marker: PhantomData<Source>,
}

impl<TRecord, Source, E> Iterator for Parser<TRecord, Source, E>
where
    TRecord: Readable<Source, E>,
    Source: Read,
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

impl<TRecord, Source, E> Parser<TRecord, Source, E>
where
    TRecord: Readable<Source, E>,
    Source: Read,
    E: Error,
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

pub trait Writable<E: Error> {
    #[doc(hidden)]
    fn write_header<W: Write>(writer: &mut W) -> Result<(), E>;

    #[doc(hidden)]
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), E>;
}

pub struct Serializer<TRecord, Target, E>
where
    TRecord: Writable<E>,
    Target: Write,
    E: Error,
{
    target: Target,
    _marker: PhantomData<(TRecord, E)>,
}

impl<TRecord, Target, E> Serializer<TRecord, Target, E>
where
    TRecord: Writable<E>,
    Target: Write,
    E: Error,
{
    pub fn new(target: Target) -> Self {
        Self {
            target,
            _marker: PhantomData,
        }
    }

    pub fn serialize(&mut self, records: &[TRecord]) -> Result<(), E> {
        TRecord::write_header(&mut self.target)?;

        for record in records {
            record.write(&mut self.target)?;
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn into_inner(self) -> Target {
        self.target
    }
}