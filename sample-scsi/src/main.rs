extern crate smart;
use smart::scsi;
use smart::data::inquiry;
use smart::data::vpd::device_id;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

fn print_hex(data: &[u8]) {
	for i in 0..data.len() {
		if i % 16 == 0 { print!("\n"); }
		print!(" {:02x}", data[i]);
	}
	print!("\n");
}

fn query(what: &str, file: &str, vpd: bool, page: u8) -> [u8; 4096] {
	print!("=== {} ===\n", what);
	let (sense, data) = scsi::scsi_inquiry(&file, vpd, page).unwrap();

	print!("sense:");
	print_hex(&sense);

	print!("data:");
	print_hex(&data);

	data
}

fn main() {
	let args = App::new("sample-scsi")
		.version(crate_version!())
		.arg(Arg::with_name("device")
			.help("Device to query")
			.required(true)
			.index(1)
		)
		.get_matches();

	let file = args.value_of("device").unwrap();

	let data = query("Inquiry", &file, false, 0);
	print!("{:#?}\n", inquiry::parse_inquiry(&data));

	let data = query("[00] Supported VPD pages", &file, true, 0);
	let len = data[3];
	print!("supported:");
	for i in 0..len {
		print!(" {:02x}", data[(i+4) as usize]);
	}
	print!("\n");

	let data = query("[83] Device Information", &file, true, 0x83);
	let len = ((data[2] as usize) << 8) + (data[3] as usize);

	print!("descriptors: {:#?}\n", device_id::parse(&data[4 .. 4+len]));
}
