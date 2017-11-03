use ata;

// SFF-8035i rev 2, 2.8 S.M.A.R.T. RETURN STATUS
pub fn parse_smart_status<'a>(reg: &'a ata::RegistersRead) -> Option<bool> {
	match (reg.cyl_low, reg.cyl_high) {
		(0x4f, 0xc2) => Some(true),
		(0xf4, 0x2c) => Some(false),
		_ => None, // WTF
	}
}
