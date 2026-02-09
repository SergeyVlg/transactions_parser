mod txt_format;

use std::error::Error;
use std::io::{Read, Write};

trait Parser<E: Error> : Sized {
    fn from_read<R: Read>(reader: &mut R) -> Result<Self, E>;
    fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), E>;
}