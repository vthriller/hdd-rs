mod parser;
mod presets;
pub use self::parser::Entry;

use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;

use std::{error, fmt, convert};

#[derive(Debug)]
pub enum Error {
	IO(io::Error),
	Parse, // TODO? Parse(nom::verbose_errors::Err) if dependencies.nom.features = ["verbose-errors"]
}
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::IO(ref err) => write!(f, "IO error: {}", err),
			Error::Parse => write!(f, "Parse error"),
		}
	}
}
impl error::Error for Error {
	fn description(&self) -> &str {
		match *self {
			Error::IO(ref err) => err.description(),
			Error::Parse => "malformed database",
		}
	}
	fn cause(&self) -> Option<&error::Error> {
		match *self {
			Error::IO(ref err) => Some(err),
			Error::Parse => None,
		}
	}
}
impl convert::From<io::Error> for Error {
	fn from(err: io::Error) -> Error { Error::IO(err) }
}

pub fn load(file: &str) -> Result<Vec<Entry>, Error> {
	let mut db = Vec::new();
	File::open(&file)?.read_to_end(&mut db)?;

	match parser::database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}
