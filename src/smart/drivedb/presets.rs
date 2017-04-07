use std::str;

use nom;
use nom::digit;

#[derive(Debug, Clone, PartialEq)]
pub enum Type { HDD, SSD }
#[derive(Debug, Clone)]
pub struct Attribute {
	pub id: u8,
	pub name: Option<String>,
	pub format: String,
	pub byte_order: String,
	pub drivetype: Option<Type>,
}

fn not_comma(c: u8) -> bool { c == ',' as u8 }
fn not_comma_nor_colon(c: u8) -> bool { c == ',' as u8 || c == ':' as u8 }

// parse argument of format 'ID,FORMAT[:BYTEORDER][,NAME[,(HDD|SSD)]]'
// `opt!()` is used with `complete!()` here because the former returns `Incomplete` untouched, thus making attributes not ending with otherwise optional ',(HDD|SSD)' `Incomplete` as well.
// TODO:
// > If 'N' is specified as ID, the settings for all Attributes are changed.
named!(pub parse_vendor_attribute <Attribute>, do_parse!(
	id: map!(digit, |x: &[u8]| str::from_utf8(x).unwrap().parse::<u8>().unwrap()) >> // XXX map_res!()?
	char!(',') >>
	format: map_res!(
		take_till1_s!(not_comma_nor_colon),
		str::from_utf8
	) >>
	byte_order: opt!(complete!(do_parse!( // TODO len()<6 should be invalid
		char!(':') >>
		byteorder: map_res!(
			take_till1_s!(not_comma),
			str::from_utf8
		) >>
		(byteorder)
	))) >>
	name_drive_type: opt!(complete!(do_parse!(
		char!(',') >>
		name: map_res!(
			take_till1_s!(not_comma),
			str::from_utf8
		) >>
		drive_type: opt!(complete!(do_parse!(
			char!(',') >>
			drive_type: alt!(tag!("HDD") | tag!("SSD")) >>
			(match str::from_utf8(drive_type) {
				Ok("HDD") => Type::HDD,
				Ok("SSD") => Type::SSD,
				_ => unreachable!(),
			})
		))) >>
		(name, drive_type)
	))) >>
	eof!() >>
	({
		let (name, drive_type) = match name_drive_type {
			Some((name, drive_type)) => (Some(name), drive_type),
			None => (None, None),
		};
		let default_byte_order = match format.as_ref() {
			// default byte orders, from ata_get_attr_raw_value, atacmds.cpp
			"raw64" | "hex64" => "543210wv",
			"raw56" | "hex56" | "raw24/raw32" | "msec24hour32" => "r543210",
			_ => "543210",
		};
		Attribute {
			id: id,
			name: name.map(|x| x.to_string()),
			format: format.to_string(),
			byte_order: byte_order.unwrap_or(default_byte_order).to_string(),
			drivetype: drive_type,
		}
	})
));

pub fn parse(line: &String) -> Option<Vec<Attribute>> {
	// using clap here would be an overkill
	let mut args = line.split_whitespace().into_iter();
	let mut output = Vec::<Attribute>::new();
	loop {
		match args.next() {
			None => return Some(output),
			Some(key) => match args.next() {
				None => return None, // we always expect an argument for the option
				Some(value) => {
					match key {
						// FIXME strings to bytes to strings again… sounds really stupid
						"-v" => match parse_vendor_attribute(value.as_bytes()) {
							nom::IResult::Done(_, attr) => { output.push(attr); },
							nom::IResult::Error(_) => (), // TODO?
							nom::IResult::Incomplete(_) => (), // TODO?
						},
						_ => continue, // TODO other options
					}
				},
			},
		}
	}
}
