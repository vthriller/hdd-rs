#[derive(Debug)]
pub struct SmartAttribute<'a> {
	id: u8,
	pre_fail: bool, // if true, failure is predicted within 24h; otherwise, attribute indicates drive's exceeded intended design life period
	online: bool,
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
		attrs.push(SmartAttribute {
			id: data[offset],
			pre_fail: data[offset + 1] & (1<<0) != 0,
			online: data[offset + 1] & (1<<1) != 0,
			flags: ((data[offset + 1] & !(0b11)) as u16) + ((data[offset + 2] as u16) << 8), // XXX endianness?
			value: data[offset + 3],
			worst: data[offset + 4],
			raw: &data[offset + 5 .. offset + 12],
		})
	}
	attrs
}
