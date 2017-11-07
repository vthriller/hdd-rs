use std::cmp::{min, max};
use drivedb;
use std::fmt;

// Initially I used `BigEndian` from the `byteorder` crate; however, it quickly resulted in an iterator mess (`.chunks()`, `.take()`, `.skip()`, `.map()`, `.unwrap()` et al.), and it also did not help with 24-bit and 48-bit packed values at all.
fn read(data: &[u8], bits: usize) -> u64 {
	let mut out: u64 = 0;
	for i in 0..(bits/8) {
		out <<= 8;
		out += data[i] as u64;
	}
	out
}

#[derive(Serialize, Debug)]
pub enum Raw {
	Raw8(Vec<u8>),
	Raw16(Vec<u16>),
	Raw64(u64),
	Raw16opt16(u16, Option<Vec<u16>>),
	Raw16avg16 { value: u16, average: u16 },
	Raw24opt8(u32, Option<Vec<u8>>),
	Raw24div(u32, u32),
	Minutes(u64),
	Seconds(u64),
	HoursMilliseconds(u32, u32),
	Celsius(f32),
	CelsiusMinMax { current: u8, min: u8, max: u8 },
}

fn write_vec<T>(f: &mut fmt::Formatter, vec: &Vec<T>) -> fmt::Result
where T: fmt::Display {
	let mut values = vec.iter();
	if let Some(i) = values.next() { write!(f, "{}", i)?; }
	for i in values { write!(f, " {}", i)?; }
	Ok(())
}

impl fmt::Display for Raw {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::Raw::*;
		match *self {
			Raw8(ref vals) => write_vec(f, &vals),
			Raw16(ref vals) => write_vec(f, &vals),
			Raw64(val) => write!(f, "{}", val),
			Raw16opt16(ref x, ref y) => {
				write!(f, "{}", x)?;
				if let &Some(ref vec) = y {
					write!(f, " (")?;
					write_vec(f, &vec)?;
					write!(f, ")")?;
				}
				Ok(())
			}
			Raw24opt8(ref x, ref y) => {
				write!(f, "{}", x)?;
				if let &Some(ref vec) = y {
					write!(f, " (")?;
					write_vec(f, &vec)?;
					write!(f, ")")?;
				}
				Ok(())
			}
			Raw16avg16 { value, average } =>
				write!(f, "{} (avg: {})", value, average),
			Raw24div(x, y) => write!(f, "{}/{}", x, y),
			Minutes(m) => {
				let h = m / 60; let m = m - h * 60;
				let d = h / 24; let h = h - d * 24;

				write!(f, "{}d {:02}:{:02}", d, h, m)
			},
			Seconds(s) => {
				let m = s / 60; let s = s - m * 60;
				let h = m / 60; let m = m - h * 60;
				let d = h / 24; let h = h - d * 24;

				write!(f, "{}d {:02}:{:02}:{:02}", d, h, m, s)
			},
			HoursMilliseconds(h, ms) => {
				let s = ms as f32 / 1000.;
				let m = s as u32 / 60; let s = s - (m as f32) * 60.;
				let d = h / 24; let h = h - d * 24;

				write!(f, "{}d {:02}:{:02}:{:02}", d, h, m, s)
			},
			Celsius(cur) => write!(f, "{:.1}°C", cur), // .1 because f32
			CelsiusMinMax { current, min, max } =>
				write!(f, "{}°C (min: {}°C, max: {}°C)", current, min, max),
		}
	}
}

// In smartmontools, they first apply byte order attribute to get the u64, which in turn is used to get separate u8/u16s for RAWFMT_RAW8/RAWFMT_RAW16; there's also different byte order defaults for different formats. Oh for crying out loud…

// `data` is a slice that contains all the attribute data, including attribute id
fn reorder(data: &[u8], byte_order: &str) -> Vec<u8> {
	byte_order.chars().map(|c| match c {
		'v' => data[3], // value
		'w' => data[4], // worst
		'0' => data[5],
		'1' => data[6],
		'2' => data[7],
		'3' => data[8],
		'4' => data[9],
		'5' => data[10],
		'r' => data[11], // reserved byte
		// smartmontools defaults to 0 for any unrecognized character;
		// we'll use '_' later for padding
		_ => 0,
	}).collect()
}

impl Raw {
	// `data`: see above
	pub fn from_raw_entry(data: &[u8], attr: &Option<drivedb::Attribute>) -> Self {
		let (fmt, byte_order) = attr.as_ref().map(|a|
			(a.format.clone(), a.byte_order.clone())
		).unwrap_or(
			("raw48".to_string(), "543210".to_string())
		);
		let raw48 = reorder(&data, &byte_order);
		let raw64 = reorder(&data, &format!("{:_>8}", byte_order));

		use self::Raw::*;
		match fmt.as_ref() {
			"raw8" => Raw8(raw48),
			"raw16" => Raw16(
				raw48.chunks(2).map(|i| read(i, 16) as u16).collect()
			),
			"raw56" | "hex56" => Raw64(read(&raw64[1..8], 56)),
			"raw64" | "hex64" => Raw64(read(&raw64, 64)),
			"raw16(avg16)" => {
				let mut raw48 = raw48.chunks(2).map(|i| read(i, 16) as u16);
				Raw16avg16 {
					value: raw48.next().unwrap(), // the 0th element should always be here
					average: raw48.next().unwrap(), // and so is the 1st
				}
			},
			"raw16(raw16)" => {
				let mut raw48 = raw48.chunks(2).map(|i| read(i, 16) as u16);
				let x = raw48.next().unwrap(); // 0th element should always be here
				let opt: Vec<u16> = raw48.collect();
				Raw16opt16(x, opt.iter().filter(|&&i| i>0).max().map(|_| opt.clone()))
			},
			"raw24(raw8)" => {
				let x = read(&raw48[3..6], 24) as u32;
				let opt: Vec<u8> = raw48.iter().take(3).map(|&i|i).collect();
				Raw24opt8(x, opt.iter().filter(|&&i| i>0).max().map(|_| opt.clone()))
			},
			"raw24/raw24" => Raw24div(
				read(&raw48[0..3], 24) as u32,
				read(&raw48[3..6], 24) as u32,
			),
			"raw24/raw32" => Raw24div(
				read(&raw64[1..4], 24) as u32,
				read(&raw64[4..8], 32) as u32,
			),
			"sec2hour" => Seconds(read(&raw48, 48)),
			"min2hour" => Minutes(read(&raw48, 48)),
			"halfmin2hour" => Seconds(read(&raw48, 48) * 30),
			"msec24hour32" => HoursMilliseconds(
				read(&raw64[4..8], 32) as u32, // hour
				read(&raw64[1..4], 24) as u32, // msec
			),
			"temp10x" => Celsius(
				read(&raw48[4..6], 16) as f32 / 10.
			),
			"tempminmax" => {
				/*
				This is a chart of all possible raw value interpretations for attributes of type 'tempminmax':
				> [5][4][3][2][1][0] raw[]
				> [ 2 ] [ 1 ] [ 0 ]  word[]
				> xx HH xx LL xx TT (Hitachi/HGST)
				> xx LL xx HH xx TT (Kingston SSDs)
				> 00 00 HH LL xx TT (Maxtor, Samsung, Seagate, Toshiba)
				> 00 00 00 HH LL TT (WDC)
				> CC CC HH LL xx TT (WDC, CCCC=over temperature count)
				> (xx = 00/ff, possibly sign extension of lower byte)

				However, smartmontools itself looks for slightly generalized patterns, in order:
				00 00 00 00 xx TT
				00 00 HL LH xx TT
				00 00 00 HL LH TT
				xx HL xx LH xx TT
				CC CC HL LH xx TT

				TODO? negative temperatures (see e.g. https://www.smartmontools.org/ticket/291)
				TODO? we're also skipping WDC overheating counters here
				*/

				// #![feature(slice_patterns)] is only available in nightly Rust, so here goes some ugliness
				let mut r = raw48.iter();
				let r = (
					*r.next().unwrap(),
					*r.next().unwrap(),
					*r.next().unwrap(),
					*r.next().unwrap(),
					*r.next().unwrap(),
					*r.next().unwrap(),
				);
				match r {
					(0, 0, 0, 0, 0, t) => Celsius(t as f32),
					(0, 0, 0, x, y, t) => CelsiusMinMax {
						current: t,
						min: min(x, y),
						max: max(x, y),
					},
					(0, 0, x, y, 0, t) => CelsiusMinMax {
						current: t,
						min: min(x, y),
						max: max(x, y),
					},
					(0, x, 0, y, 0, t) => CelsiusMinMax {
						current: t,
						min: min(x, y),
						max: max(x, y),
					},
					// whatever this might be, show it using default formatter
					_ => Raw64( read(&raw48, 48) ),
				}
			},
			// {raw,hex}48 is the default
			_ => Raw64( read(&raw48, 48) ),
		}

	}
}
