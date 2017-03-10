#[derive(Debug)]
pub struct SmartAttribute<'a> {
	id: u8,

	pre_fail: bool, // if true, failure is predicted within 24h; otherwise, attribute indicates drive's exceeded intended design life period
	online: bool,
	// In SFF-8035i rev 2, bits 2-5 are defined as vendor-specific, and 6-15 are reserved;
	// however, these days the following seems to be universally interpreted the way it was once (probably) established by IBM, Maxtor and Quantum
	performance: bool,
	error_rate: bool,
	event_count: bool,
	self_preserving: bool,
	flags: u16,

	value: u8, // TODO? 0x00 | 0xfe | 0xff are invalid
	// vendor-specific:
	worst: u8,
	raw: &'a [u8], // including the last byte, which is reserved
}

pub fn parse_smart_values<'a>(data: &'a [u8; 512]) -> Vec<SmartAttribute<'a>> {
	// TODO cover bytes 0..1 362..511
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
		})
	}
	attrs
}
