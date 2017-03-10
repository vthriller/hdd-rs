use std::fs::File;

extern crate smart;
use smart::ata;
use smart::data::id;
use smart::data::attr;

fn main() {
	let file = File::open("/dev/sda").unwrap();

	let data = ata::ata_exec(&file, ata::WIN_IDENTIFY, 1, 0, 1).unwrap();
	print!("{:?}\n", id::parse_id(data));

	let data = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_VALUES, 1).unwrap();
	print!("{:?}\n", attr::parse_smart_values(&data));
}
