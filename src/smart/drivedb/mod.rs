mod parser;
mod presets;
pub mod vendor_attribute;
pub use self::parser::Entry;
pub use self::vendor_attribute::Attribute;

use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;

use std::{error, fmt, convert};

use super::data::id;

use regex::bytes::Regex;

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

// TODO load_compiled, with pre-compiled headers and pre-parsed presets,
// for those who work with drives in bulk
// TODO invalid regex should result in parsing error (or maybe not, maybe just stick to Option<Regex>)
pub fn load(file: &str) -> Result<Vec<Entry>, Error> {
	let mut db = Vec::new();
	File::open(&file)?.read_to_end(&mut db)?;

	match parser::database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}

fn filter_presets(id: &id::Id, preset: Vec<Attribute>) -> Vec<Attribute> {
	let drivetype = match id.rpm {
		id::RPM::RPM(_) => Some(vendor_attribute::Type::HDD),
		id::RPM::NonRotating => Some(vendor_attribute::Type::SSD),
		id::RPM::Unknown => None,
	};

	preset.into_iter().filter(|ref attr| match (&attr.drivetype, &drivetype) {
		// this attribute is not type-specific
		(&None, _) => true,
		// drive type match
		(&Some(ref a), &Some(ref b)) if a == b => true,
		// drive type does not match
		(&Some(_), &Some(_)) => false,
		// applying drive-type-specific attributes to drives of unknown type makes no sense
		(&Some(_), &None) => false,
	}).collect()
}

#[derive(Debug)]
pub struct Match<'a> {
	pub family: Option<&'a String>,
	pub warning: Option<&'a String>,
	pub presets: Vec<Attribute>,
}

// FIXME extra_attributes should probably be the reference
pub fn match_entry<'a>(id: &id::Id, db: &'a Vec<Entry>, extra_attributes: Vec<Attribute>) -> Match<'a> {
	let mut db = db.iter();

	let default = db.next().unwrap(); // svn-id entry should always be present …I guess. TODO, but only when we decide to support '-B +FILE' switch
	let default = if default.presets == "" {
		db.next().unwrap() // I'm fine with panicking in the absence of default entry (XXX)
	} else {
		// older drivedb versions included default settings not as a separate entry, but in a comment as part of svn-id entry
		default
	};

	for entry in db {

		// USB ID entries are parsed differently; also, we don't support USB devices yet
		if entry.model.starts_with("USB:") { continue }

		// model and firmware are expected to be ascii strings, no need to try matching unicode characters

		// > [modelregexp] should never be "".
		let re = Regex::new(format!("(?-u)^{}$", entry.model).as_str()).unwrap();
		if !re.is_match(id.model.as_bytes()) { continue }

		if entry.firmware.len() > 0 {
			let re = Regex::new(format!("^(?-u){}$", entry.firmware).as_str()).unwrap();
			if !re.is_match(id.firmware.as_bytes()) { continue }
		}

		// > The table will be searched from the start to end or until the first match
		return Match {
			family: Some(&entry.family),
			warning: if entry.warning.len() > 0 { Some(&entry.warning) } else { None },
			presets: filter_presets(&id, vendor_attribute::merge(&vec![
				presets::parse(&default.presets),
				presets::parse(&entry.presets),
				Some(extra_attributes),
			])),
		};
	}

	Match {
		family: None,
		warning: None,
		presets: filter_presets(&id, vendor_attribute::merge(&vec![
			presets::parse(&default.presets),
			Some(extra_attributes),
		])),
	}
}

impl<'a> Match<'a> {
	pub fn render_attribute(&'a self, id: u8) -> Option<Attribute> {
		vendor_attribute::render(self.presets.to_vec(), id)
	}
}
