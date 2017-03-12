use std::fs::File;

extern crate smart;
use smart::ata;
use smart::data::id;
use smart::data::attr;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

fn bool_to_sup(b: bool) -> &'static str {
	match b {
		false => "not supported",
		true  => "supported",
	}
}

fn bool_to_flag(b: bool, c: char) -> char {
	if b { c } else { '-' }
}

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
			if id.incomplete { print!("WARNING: device reports information it provides is incomplete\n\n"); }

			// XXX id.is_ata is deemed redundant and is skipped
			// XXX we're skipping id.commands_supported for now as it is hardly of any interest to users

			print!("Model:    {}\n", id.model);
			print!("Firmware: {}\n", id.firmware);
			print!("Serial:   {}\n", id.serial);
			// TODO: id.wwn_supported is cool, but actual WWN ID is better

			print!("\n");

			print!("ATA version:\n{}\n", id.ata_version.unwrap_or("unknown"));

			print!("\n");

			// The following guide, when printed, is exactly 80 characters
			// ... "..............................................................supported disabled\n"
			print!("Host protected area:           {}\n", id.hpa);
			print!("Advanced Power Management:     {}\n", id.apm);
			print!("Automatic Acoustic Management: {}\n", id.aam);
			print!("Read look-ahead:               {}\n", id.read_look_ahead);
			print!("Write cache:                   {}\n", id.write_cache);
			print!("Power management:              {}\n", bool_to_sup(id.power_mgmt_supported));
			print!("General purpose logging:       {}\n", bool_to_sup(id.gp_logging_supported));
			print!("Trusted computing:             {}\n", bool_to_sup(id.trusted_computing_supported));
			print!("ATA security:                  {}\n", id.security);

			print!("\n");

			print!("S.M.A.R.T.:    {}\n", id.smart);
			print!("Error logging: {}\n", bool_to_sup(id.smart_error_logging_supported));
			print!("Self-test:     {}\n", bool_to_sup(id.smart_self_test_supported));

			print!("\n");
		}

		if print_attrs {
			if id.smart == id::Ternary::Enabled {
				let data = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_VALUES, 1).unwrap();
				let thresh = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_THRESHOLDS, 1).unwrap();

				let values = attr::parse_smart_values(&data, &thresh);

				// TODO attribute names
				// TODO when-failed (now/past/never)

				print!("S.M.A.R.T. attribute values:\n");
				print!(" ID flags        value worst thresh raw\n");
				for val in values {
					print!("{:3} {}{}{}{}{}{} {:04x}    {:3}   {:3}    {} {:?}\n",
						val.id,
						bool_to_flag(val.pre_fail, 'P'),
						bool_to_flag(val.online, 'O'),
						bool_to_flag(val.performance, 'S'),
						bool_to_flag(val.error_rate, 'R'),
						bool_to_flag(val.event_count, 'C'),
						bool_to_flag(val.self_preserving, 'K'),
						val.flags,
						val.value, val.worst,
						match val.thresh {
							Some(t) => format!("{:3}", t),
							None => "?".to_string(),
						},
						// TODO interpreted raw values
						val.raw,
					);
				}
				// based on the output of 'smartctl -A -f brief' (part of 'smartctl -x')
				print!("    │││││└─ K auto-keep\n");
				print!("    ││││└── C event count\n");
				print!("    │││└─── R error rate\n");
				print!("    ││└──── S speed/performance\n");
				print!("    │└───── O updated online\n");
				print!("    └────── P prefailure warning\n");
			} else {
				print!("S.M.A.R.T. is disabled, cannot show attributes\n")
			}
		}
	}
}
