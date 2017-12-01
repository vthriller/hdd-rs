#[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
pub fn hexdump(data: &[u8]) -> String {
	let mut dump = String::with_capacity(3*data.len() + data.len()/16 + 2);
	for i in 0..data.len() {
		if i % 16 == 0 {
			dump.push('\n');
		}
		dump.push_str(&format!(" {:02x}", data[i]));
	}
	dump.push('\n');
	dump
}
