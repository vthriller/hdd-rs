// SFF-8035i rev 2, 2.8 S.M.A.R.T. RETURN STATUS
pub fn parse_smart_status<'a>(data: &'a [u8; 7]) -> Option<bool> {
	match (data[4], data[5]) { // (lcyl, hcyl)
		(0x4f, 0xc2) => Some(true),
		(0xf4, 0x2c) => Some(false),
		_ => None, // WTF
	}
}
