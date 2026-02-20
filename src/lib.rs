mod txt_format;

use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};

struct Parser<TRecord : Parsable<ReadError, WriteError>> {
    reader: Box<dyn Read>,
    writer: Box<dyn Write>,
}


trait Parsable<ReadError: Error, WriteError: Error> : Sized {
    fn from_read<R: Read>(reader: &mut R) -> Result<Self, ReadError>;
    fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), WriteError>;
}