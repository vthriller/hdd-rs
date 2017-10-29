use nom::multispace;

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

/// drivedb.h entry
#[derive(Debug)]
pub struct Entry {
	/// > Informal string about the model family/series of a device.
	pub family: String,

	/// > POSIX extended regular expression to match the model of a device.
	/// > This should never be "".
	pub model: String,

	/// > POSIX extended regular expression to match a devices's firmware.
	///
	/// Optional if "".
	pub firmware: String,

	/// > A message that may be displayed for matching drives.
	/// > For example, to inform the user that they may need to apply a firmware patch.
	pub warning: String,

	/// > String with vendor-specific attribute ('-v') and firmware bug fix ('-F') options.
	/// > Same syntax as in smartctl command line.
	pub presets: String,
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

named!(pub database <Vec<Entry>>, do_parse!(
	whitespace >>
	entries: many1!(do_parse!(
		e: entry >> comma >> (e)
	)) >>
	whitespace >>
	eof!() >>
	(entries.into_iter().filter(|entry| {
		// > The entry is ignored if [modelfamily] starts with a dollar sign.
		!entry.family.starts_with('$')
	}).collect())
));
