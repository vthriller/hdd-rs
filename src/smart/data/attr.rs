use std::collections::HashMap;

#[derive(Debug)]
pub struct SmartAttribute<'a> {
	pub id: u8,

	pub pre_fail: bool, // if true, failure is predicted within 24h; otherwise, attribute indicates drive's exceeded intended design life period
	pub online: bool,
	// In SFF-8035i rev 2, bits 2-5 are defined as vendor-specific, and 6-15 are reserved;
	// however, these days the following seems to be universally interpreted the way it was once (probably) established by IBM, Maxtor and Quantum
	pub performance: bool,
	pub error_rate: bool,
	pub event_count: bool,
	pub self_preserving: bool,
	pub flags: u16,

	pub value: u8, // TODO? 0x00 | 0xfe | 0xff are invalid
	// vendor-specific:
	pub worst: u8,
	pub raw: &'a [u8], // including the last byte, which is reserved

	pub thresh: Option<u8>, // requested separately; TODO? 0x00 is "always passing", 0xff is "always failing", 0xfe is invalid
}

pub fn parse_smart_values<'a>(data: &'a [u8; 512], raw_thresh: &'a [u8; 512]) -> Vec<SmartAttribute<'a>> {
	// TODO cover bytes 0..1 362..511 of data
	// XXX what if some drive reports the same attribute multiple times?

	let mut threshs = HashMap::<u8, u8>::new();
	for i in 0..30 {
		let offset = 2 + i * 12;
		if raw_thresh[offset] == 0 { continue } // attribute table entry of id 0x0 is invalid
		threshs.insert(raw_thresh[offset], raw_thresh[offset+1]);
		// fields 2..11 are reserved
	}

	let mut attrs = vec![];
	for i in 0..30 {
		let offset = 2 + i * 12;
		if data[offset] == 0 { continue } // attribute table entry of id 0x0 is invalid

		let flags = (data[offset + 1] as u16) + ((data[offset + 2] as u16) << 8); // XXX endianness?

		attrs.push(SmartAttribute {

			id: data[offset],
			pre_fail:        flags & (1<<0) != 0,
			online:          flags & (1<<1) != 0,
			performance:     flags & (1<<2) != 0,
			error_rate:      flags & (1<<3) != 0,
			event_count:     flags & (1<<4) != 0,
			self_preserving: flags & (1<<5) != 0,
			flags:           flags & (!0b111111),

			value: data[offset + 3],
			worst: data[offset + 4],
			raw: &data[offset + 5 .. offset + 12],

			// .get() returns Option<&T>, but threshs would not live long enough, and it's just easier to copy u8 using this map
			thresh: threshs.get(&data[offset]).map(|t| *t),
		})
	}
	attrs
}
