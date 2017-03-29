use std::fs::File;
use std::io::prelude::*;
use std::io;

use nom;
use nom::multispace;

use std::{error, fmt, convert};

named!(comment_block, do_parse!(
	tag!("/*") >>
	take_until!("*/") >>
	tag!("*/") >>
	(&[])
));

named!(comment, do_parse!(
	tag!("//") >>
	take_until!("\n") >>
	char!('\n') >>
	(&[])
));

named!(whitespace, do_parse!(
	many0!(alt!(
		multispace | comment | comment_block
	)) >>
	(&[])
));

// TODO? \[bfav?0] \ooo \xhh
named!(string_escaped_char <char>, do_parse!(
	char!('\\') >>
	s: map!(one_of!("\\\"'nrt"), |c| match c {
		'\\' => '\\',
		'"' => '"',
		'\'' => '\'',
		'n' => '\n',
		'r' => '\r',
		't' => '\t',
		_ => unreachable!(),
	}) >>
	(s)
));

named!(string_char <char>, alt!(
	none_of!("\n\\\"")
	| string_escaped_char
));

named!(string_literal <String>, do_parse!(
	char!('\"') >>
	s: map!(
		many0!(string_char),
		|s: Vec<char>| { s.into_iter().collect() }
	) >>
	char!('\"') >>
	(s)
));

named!(string <String>, do_parse!(
	s0: string_literal >>
	ss: many0!(do_parse!(
		whitespace >>
		s: string_literal >>
		(s)
	)) >>
	({
		let mut s = s0.to_owned();
		for i in ss { s.push_str(i.as_str()) }
		s
	})
));

#[derive(Debug)]
pub struct Entry {
	family: String,
	model: String,
	firmware: String,
	warning: String,
	presets: String,
}

named!(comma, do_parse!(whitespace >> char!(',') >> whitespace >> (&[])));

named!(entry <Entry>, do_parse!(
	char!('{') >> whitespace >>
	family: string >> comma >>
	model: string >> comma >>
	firmware: string >> comma >>
	warning: string >> comma >>
	presets: string >> whitespace >>
	char!('}') >>
	(Entry {
		family: family,
		model: model,
		firmware: firmware,
		warning: warning,
		presets: presets,
	})
));

named!(database <Vec<Entry>>, do_parse!(
	whitespace >>
	entries: many1!(do_parse!(
		e: entry >> comma >> (e)
	)) >>
	whitespace >>
	eof!() >>
	(entries)
));

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

	match database(&db) {
		nom::IResult::Done(_, entries) => Ok(entries),
		nom::IResult::Error(_) => Err(Error::Parse),
		nom::IResult::Incomplete(_) => unreachable!(), // XXX is it true?
	}
}
