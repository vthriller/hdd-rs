use std::fs::File;

extern crate smart;

fn main() {
	let data = smart::identify(
		File::open("/dev/sda").unwrap()
	).unwrap();

	print!("{:?}\n", smart::parse_id(data));
}
