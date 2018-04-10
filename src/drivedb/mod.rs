/*!
Use this module to match hard drive and SMART values it returns against smartmontools database.

## Example

```
use hdd::drivedb;
use hdd::drivedb::vendor_attribute;

// look for version updated with `update-smart-drivedb(8)` first
let drivedb = drivedb::load("/var/lib/smartmontools/drivedb/drivedb.h").or(
	drivedb::load("/usr/share/smartmontools/drivedb.h")
)?;

// extra attribute definitions that user might give
let user_attributes = vec!["9,minutes"]
	.into_iter()
	.map(|attr| vendor_attribute::parse(attr).unwrap())
	.collect();

// TODO: issue ATA IDENTIFY DEVICE cmd and parse the answer here
let id = unimplemented!();

let dbentry = drivedb::match_entry(
	&id,
	&drivedb,
	user_attributes,
);

if let Some(warn) = dbentry.warning {
	println!("WARNING: {}", warn);
}
```
*/

mod parser;
mod presets;
pub mod vendor_attribute;
pub use self::parser::Entry;
pub use self::vendor_attribute::Attribute;

use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;

use ata::data::id;

use regex::bytes::Regex;

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

// TODO load_compiled, with pre-compiled headers and pre-parsed presets,
// for those who work with drives in bulk
// TODO invalid regex should result in parsing error (or maybe not, maybe just stick to Option<Regex>)
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

fn filter_presets(id: &id::Id, preset: Vec<Attribute>) -> Vec<Attribute> {
	let drivetype = {
		use self::id::RPM::*;
		use self::vendor_attribute::Type::*;
		match id.rpm {
			RPM(_) => Some(HDD),
			NonRotating => Some(SSD),
			Unknown => None,
		}
	};

	#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
	preset.into_iter().filter(|attr| match (&attr.drivetype, &drivetype) {
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

/// Drive-related data that cannot be queried from the drive itself (model family, attribute presets etc.)
#[derive(Debug)]
pub struct DriveMeta<'a> {
	/// > Informal string about the model family/series of a device.
	pub family: Option<&'a String>,

	/// > A message that may be displayed for matching drives.
	/// > For example, to inform the user that they may need to apply a firmware patch.
	pub warning: Option<&'a String>,

	/// SMART attribute descriptions
	pub presets: Vec<Attribute>,
}

/// Returns default entry from the database (if any).
pub fn get_default_entry(db: &Vec<Entry>) -> Option<&Entry> {
	for entry in db.iter() {
		if entry.family == "DEFAULT" {
			return Some(entry)
		}
	}
	return None
}

fn match_drive<'a>(id: &id::Id, db: &'a Vec<Entry>) -> Option<&'a Entry> {
	for entry in db.iter() {
		// USB ID entries are parsed differently; also, we don't support USB devices yet
		if entry.model.starts_with("USB:") { continue }

		// model and firmware are expected to be ascii strings, no need to try matching unicode characters

		// > [modelregexp] should never be "".
		let re = Regex::new(format!("(?-u)^{}$", entry.model).as_str()).unwrap();
		if !re.is_match(id.model.as_bytes()) { continue }

		if ! entry.firmware.is_empty() {
			let re = Regex::new(format!("^(?-u){}$", entry.firmware).as_str()).unwrap();
			if !re.is_match(id.firmware.as_bytes()) { continue }
		}

		return Some(entry);
	}

	None
}

// FIXME extra_attributes should probably be the reference
/**
Matches given ATA IDENTIFY DEVICE response `id` against drive database `db`.

Return value is a merge between the default entry and the first match; if multiple entries match the `id`, the first one is used (this is consistent with smartmontools' `lookup_drive` function).
`extra_attributes` are also appended to the list of presets afterwards.

This functions skips USB ID entries.
*/
pub fn render_meta<'a>(id: &id::Id, db: &'a Vec<Entry>, extra_attributes: Vec<Attribute>) -> DriveMeta<'a> {
	let default = get_default_entry(&db).unwrap(); // FIXME unwrap

	let mut m = DriveMeta {
		family: None,
		warning: None,
		presets: Vec::<Attribute>::new(),
	};
	if let Some(presets) = presets::parse(&default.presets) {
		m.presets.extend(presets);
	}

	if let Some(entry) = match_drive(id, db) {
		if let Some(presets) = presets::parse(&entry.presets) {
			m.presets.extend(presets);
		}

		m.family = Some(&entry.family);
		m.warning = if ! entry.warning.is_empty() { Some(&entry.warning) } else { None };
	}

	m.presets.extend(extra_attributes);
	m.presets = filter_presets(id, m.presets);
	return m;
}

impl<'a> DriveMeta<'a> {
	pub fn render_attribute(&'a self, id: u8) -> Option<Attribute> {
		vendor_attribute::render(self.presets.to_vec(), id)
	}
}
