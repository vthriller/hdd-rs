extern crate smart;
use smart::scsi;
use smart::data::inquiry;

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

	let (sense, data) = scsi::scsi_inquiry(&file, false, 0).unwrap();

	print!("sense:");
	print_hex(&sense);

	print!("data:");
	print_hex(&data);

	print!("{:#?}\n", inquiry::parse_inquiry(&data));
}
