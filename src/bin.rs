use std::fs::File;

extern crate smart;
use smart::ata;
use smart::data::id;
use smart::data::attr;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

fn main() {
	let args = App::new("smart-rs")
		.about("yet another S.M.A.R.T. querying tool")
		.version(crate_version!())
		.arg(Arg::with_name("info")
			.short("i") // smartctl-like
			.long("info") // smartctl-like
			.help("Prints a basic information about the device")
		)
		.arg(Arg::with_name("attrs")
			.short("A") // smartctl-like
			.long("attributes") // smartctl-like
			.long("attrs")
			.help("Prints a list of S.M.A.R.T. attributes")
		)
		.arg(Arg::with_name("all")
			.short("a") // smartctl-like
			.long("all") // smartctl-like
			.help("equivalent to -iA")
		)
		.arg(Arg::with_name("device")
			.help("Device to query")
			.required(true)
			.index(1)
		)
		.get_matches();

	let file = File::open(args.value_of("device").unwrap()).unwrap();

	let print_info  = args.is_present("info") || args.is_present("all");
	let print_attrs = args.is_present("attrs") || args.is_present("all");

	if print_info || print_attrs {
		let data = ata::ata_exec(&file, ata::WIN_IDENTIFY, 1, 0, 1).unwrap();
		let id = id::parse_id(&data);

		if print_info {
			print!("{:?}\n", id);
		}

		if print_attrs {
			if id.smart == id::Ternary::Enabled {
				let data = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_VALUES, 1).unwrap();
				let thresh = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_THRESHOLDS, 1).unwrap();
				print!("{:?}\n", attr::parse_smart_values(&data, &thresh));
			} else {
				print!("S.M.A.R.T. is disabled, cannot show attributes\n")
			}
		}
	}
}
