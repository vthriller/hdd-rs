pub fn bytes_to_words(data: &Vec<u8>) -> Vec<u16> {
	let mut output = vec![];

	// XXX what if `data` contains odd number of u8s?
	for i in 0 .. data.len()/2 {
		if cfg!(target_endian = "little") {
			output.push(
				((data[2 * i + 1] as u16) << 8)
				+ (data[2 * i] as u16)
			);
		} else {
			output.push(
				((data[2 * i] as u16) << 8)
				+ (data[2 * i + 1] as u16)
			);
		}
	}

	output
}

fn pretty_char_from_u8(x: u8) -> char {
	if x >= 0x20 && x <= 0x7f {
		// safety: we already checked whether the u8 is a valid ascii printable (and therefore is a valid unicode codepoint)
		unsafe { ::std::char::from_u32_unchecked(x as u32) }
	} else {
		// ' ' and '.' are ambiguous, and a string of '�'s is just unreadable
		'░'
	}
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
pub fn hexdump_8(data: &[u8]) -> String {
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
		ascii.push(pretty_char_from_u8(data[i]));
	}
	dump.push(' ');
	dump.push_str(&ascii);
	ascii.truncate(0);
	dump.push('\n');
	dump
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
pub fn hexdump_16(data: &[u16]) -> String {
	// 5× len for ' {:04x}'
	// len/8 for \n
	// len/8 for ' ' before ascii
	// 2× len for ascii
	// 2 to "round" (/8)s up and have lesser chance of reallocation
	let mut dump = String::with_capacity(7*data.len() + data.len()/4 + 2);
	let mut ascii = String::with_capacity(16);

	for i in 0..data.len() {
		if i % 8 == 0 {
			dump.push(' ');
			dump.push_str(&ascii);
			ascii.truncate(0);
			dump.push('\n');
		}
		dump.push_str(&format!(" {:04x}", data[i]));
		ascii.push(pretty_char_from_u8((data[i] >> 8) as u8));
		ascii.push(pretty_char_from_u8((data[i] & 0xff) as u8));
	}
	dump.push(' ');
	dump.push_str(&ascii);
	ascii.truncate(0);
	dump.push('\n');
	dump
}
