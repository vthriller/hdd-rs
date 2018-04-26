use super::parser::{self, Entry};
use super::DriveDB;

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

fn load(file: &str) -> Result<Vec<Entry>, Error> {
	let mut db = Vec::new();
	File::open(&file)?.read_to_end(&mut db)?;

	match parser::database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}

/// Use this helper to load entries from `drivedb.h`.
#[derive(Debug)]
pub struct Loader {
	entries: Vec<Entry>,
	additional: Vec<Entry>,
}
impl Loader {
	pub fn new() -> Self {
		Loader {
			entries: vec![],
			additional: vec![],
		}
	}
	/**
	Loads entries from main drivedb file.

	Entries from previously loaded main file will be discarded; entries from additional files will not be affected.

	## Errors

	Returns [enum Error](enum.Error.html) if:

	- it encounters any kind of I/O error,
	- drive database is malformed.
	*/
	pub fn load(&mut self, file: &str) -> Result<(), Error> {
		self.entries = load(file)?;
		Ok(())
	}
	/**
	Loads more entries from additional drivedb file. Additional entries always take precedence over the main file.

	## Errors

	Returns [enum Error](enum.Error.html) if:

	- it encounters any kind of I/O error,
	- drive database is malformed.
	*/
	pub fn load_additional(&mut self, file: &str) -> Result<(), Error> {
		self.entries = load(file)?;
		Ok(())
	}
	/// Returns all loaded entries.
	pub fn db(self) -> DriveDB {
		let entries: Vec<_> = self.additional.into_iter()
			.chain(self.entries.into_iter())
			.collect();

		DriveDB::new(entries)
	}
}
