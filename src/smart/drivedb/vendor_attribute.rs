use std::str;

use nom::digit;

#[derive(Debug, Clone, PartialEq)]
pub enum Type { HDD, SSD }
#[derive(Debug, Clone)]
pub struct Attribute {
	pub id: Option<u8>,
	pub name: Option<String>,
	pub format: String,
	pub byte_order: String,
	pub drivetype: Option<Type>,
}

fn not_comma(c: u8) -> bool { c == ',' as u8 }
fn not_comma_nor_colon(c: u8) -> bool { c == ',' as u8 || c == ':' as u8 }

// parse argument of format 'ID,FORMAT[:BYTEORDER][,NAME[,(HDD|SSD)]]'
// `opt!()` is used with `complete!()` here because the former returns `Incomplete` untouched, thus making attributes not ending with otherwise optional ',(HDD|SSD)' `Incomplete` as well.
named!(pub parse <Attribute>, do_parse!(
	id: alt!(
		// XXX map_res!()?
		map!(digit, |x: &[u8]| str::from_utf8(x).unwrap().parse::<u8>().ok())
		// > If 'N' is specified as ID, the settings for all Attributes are changed
		| do_parse!(char!('N') >> (None))
	) >>
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

pub fn render(presets: Vec<Attribute>, id: u8) -> Option<Attribute> {
	let mut out = None;

	for new in presets {
		// apply updates only if it matches the attribute id we're interested in,
		// or if it matches all the ids ('-v N,…' aka `id: None`)
		match new.id {
			Some(x) if x != id => continue,
			_ => ()
		}

		match out {
			None => { out = Some(new.clone()); },
			Some(ref mut old) => {
				old.format = new.format.clone();
				old.byte_order = new.byte_order.clone();
				if let Some(_) = new.name {
					old.name = new.name.clone();
				}
				if let Some(_) = new.drivetype {
					old.drivetype = new.drivetype.clone();
				}
			},
		}
	}

	out
}

pub fn merge(presets: Vec<Option<Vec<Attribute>>>) -> Vec<Attribute> {
	let mut output = Vec::<Attribute>::new();
	for preset in presets {
		if let Some(ref dpresets) = preset {
			output.extend(dpresets.iter().cloned());
		}
	}
	output
}
