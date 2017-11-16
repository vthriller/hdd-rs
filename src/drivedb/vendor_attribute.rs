/*!
Module to parse and manipulate attribute descriptions.

Attribute descriptions usually come from two sources:

* [drivedb.h](../index.html) entries,
* command-line arguments (`smartctl -v …`).

Format for attribute descriptions is described in [smartctl(8)](https://www.smartmontools.org/browser/trunk/smartmontools/smartctl.8.in) (option `-v`/`--vendorattribute`).
*/
use std::str;

use nom;
use nom::digit;

quick_error! {
	#[derive(Debug)]
	pub enum Error {
		Parse {
			// TODO? Parse(nom::verbose_errors::Err) if dependencies.nom.features = ["verbose-errors"]
			display("Unable to parse vendor attribute")
		}
	}
}

/// HDD or SSD
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type { HDD, SSD }

/// SMART attribute description
#[derive(Debug, Clone)]
pub struct Attribute {
	/// id of described attribute
	pub id: Option<u8>,
	/// attribute name
	pub name: Option<String>,
	/// value format, like `raw48` or `tempminmax`
	pub format: String,
	/// bytes of attribute data to make value of (usually something like `r543210`, where `r`, `v`, `w` represent reserved byte, current and worst values respectively)
	pub byte_order: String,
	/// what kind of device this description is applicable to: HDD, SSD, or both
	pub drivetype: Option<Type>,
}

fn not_comma(c: u8) -> bool { c == b',' }
fn not_comma_nor_colon(c: u8) -> bool { c == b',' || c == b':' }

// `opt!()` is used with `complete!()` here because the former returns `Incomplete` untouched, thus making attributes not ending with otherwise optional ',(HDD|SSD)' `Incomplete` as well.
named!(parse_standard <Attribute>, do_parse!(
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
	// TODO '+' for ATTRFLAG_INCREASING
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
		let default_byte_order = match format {
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

/**
Parses single attribute description (`-v` option argument).

The following formats are supported:

* `ID,FORMAT[:BYTEORDER][,NAME[,(HDD|SSD)]]`
* legacy `-v` arguments, like `9,halfminutes`
*/
pub fn parse(s: &str) -> Result<Attribute, Error> {
	let s = match s {
		"9,halfminutes" => "9,halfmin2hour,Power_On_Half_Minutes",
		"9,minutes" => "9,min2hour,Power_On_Minutes",
		"9,seconds" => "9,sec2hour,Power_On_Seconds",
		"9,temp" => "9,tempminmax,Temperature_Celsius",
		"192,emergencyretractcyclect" => "192,raw48,Emerg_Retract_Cycle_Ct",
		"193,loadunload" => "193,raw24/raw24",
		"194,10xCelsius" => "194,temp10x,Temperature_Celsius_x10",
		"194,unknown" => "194,raw48,Unknown_Attribute",
		"197,increasing" => "197,raw48+,Total_Pending_Sectors",
		"198,offlinescanuncsectorct" => "198,raw48,Offline_Scan_UNC_SectCt", // TODO? there goes some `get_unc_attr_id` reference
		"198,increasing" => "198,raw48+,Total_Offl_Uncorrectabl",
		"200,writeerrorcount" => "200,raw48,Write_Error_Count",
		"201,detectedtacount" => "201,raw48,Detected_TA_Count",
		"220,temp" => "220,tempminmax,Temperature_Celsius",
		s => s,
	};
	// FIXME strings to bytes to strings again… sounds really stupid
	match parse_standard(s.as_bytes()) {
		nom::IResult::Done(_, attr) => Ok(attr),
		nom::IResult::Error(_) => Err(Error::Parse), // TODO?
		nom::IResult::Incomplete(_) => Err(Error::Parse), // TODO?
	}
}

/**
Squashes attribute description for a particular attribute `id`.

Why not simply find the latest attribute with a given `id`?

* Description might match all attributes at once (`-v N,…`, represented with `attr.id` of `None`).
* Description might only update data format, leaving previously defined name and drive type intact.
*/
pub fn render(presets: Vec<Attribute>, id: u8) -> Option<Attribute> {
	let mut out = None;

	for new in presets {
		match new.id {
			Some(x) if x != id => continue,
			_ => ()
		}

		match out {
			None => { out = Some(new.clone()); },
			Some(ref mut old) => {
				old.format = new.format.clone();
				old.byte_order = new.byte_order.clone();
				if new.name.is_some() {
					old.name = new.name.clone();
				}
				if new.drivetype.is_some() {
					old.drivetype = new.drivetype;
				}
			},
		}
	}

	out
}

/// Concatenates lists of attribute presets.
pub fn merge(presets: Vec<Option<Vec<Attribute>>>) -> Vec<Attribute> {
	let mut output = Vec::<Attribute>::new();
	for preset in presets {
		if let Some(ref dpresets) = preset {
			output.extend(dpresets.iter().cloned());
		}
	}
	output
}
