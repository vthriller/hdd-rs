use super::parser::{self, Entry};

use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;

quick_error! {
	#[derive(Debug)]
	pub enum Error {
		IO(err: io::Error) {
			from()
			display("IO error: {}", err)
			description(err.description())
			cause(err)
		}
		Parse {
			// TODO? Parse(nom::verbose_errors::Err) if dependencies.nom.features = ["verbose-errors"]
			display("Unable to parse the drivedb")
			description("malformed database")
		}
	}
}

/**
Opens `file`, parses its content and returns it as a `Vec` of entries.

## Errors

Returns [enum Error](enum.Error.html) if:

* it encounters any kind of I/O error,
* drive database is malformed.
*/
pub fn load(file: &str) -> Result<Vec<Entry>, Error> {
	let mut db = Vec::new();
	File::open(&file)?.read_to_end(&mut db)?;

	match parser::database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}
