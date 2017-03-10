use std::fs::File;

extern crate smart;

fn main() {
	let file = File::open("/dev/sda").unwrap();

	let data = smart::ata_exec(&file, smart::WIN_IDENTIFY, 1, 0, 1).unwrap();
	print!("{:?}\n", smart::parse_id(data));

	let data = smart::ata_exec(&file, smart::WIN_SMART, 0, smart::SMART_READ_VALUES, 1).unwrap();
	print!("{:?}\n", smart::parse_smart_values(&data));
}
