#[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
pub fn hexdump(data: &[u8]) -> String {
	// 3× len for ' {:02x}'
	// len/16 for \n
	// len/16 for ' ' before ascii
	// len for ascii
	// 2 to "round" (/16)s up and have lesser chance of reallocation
	let mut dump = String::with_capacity(4*data.len() + data.len()/8 + 2);
	let mut ascii = String::with_capacity(16);

	for i in 0..data.len() {
		if i % 16 == 0 {
			dump.push(' ');
			dump.push_str(&ascii);
			ascii.truncate(0);
			dump.push('\n');
		}
		dump.push_str(&format!(" {:02x}", data[i]));
		ascii.push(
			if data[i] >= 0x20 && data[i] <= 0x7f {
				// safety: we already checked whether the u8 is a valid ascii printable (and therefore is a valid unicode codepoint)
				unsafe { ::std::char::from_u32_unchecked(data[i] as u32) }
			} else {
				// ' ' and '.' are ambiguous, and a string of '�'s is just unreadable
				'░'
			}
		);
	}
	dump.push(' ');
	dump.push_str(&ascii);
	ascii.truncate(0);
	dump.push('\n');
	dump
}
