use std::fs::File;

extern crate smart;
use smart::ata;
use smart::data::id;
use smart::data::attr;
use smart::data::health;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate serde_json;
use serde_json::value::ToJson;

extern crate separator;
use separator::Separatable;
extern crate number_prefix;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

use std::io::Write;

fn bool_to_sup(b: bool) -> &'static str {
	match b {
		false => "not supported",
		true  => "supported",
	}
}

fn bool_to_flag(b: bool, c: char) -> char {
	if b { c } else { '-' }
}

fn print_id(id: &id::Id) {
	if id.incomplete { print!("WARNING: device reports information it provides is incomplete\n\n"); }

	// XXX id.is_ata is deemed redundant and is skipped
	// XXX we're skipping id.commands_supported for now as it is hardly of any interest to users

	print!("Model:    {}\n", id.model);
	print!("Firmware: {}\n", id.firmware);
	print!("Serial:   {}\n", id.serial);
	// TODO: id.wwn_supported is cool, but actual WWN ID is better

	print!("\n");

	print!("Capacity: {} bytes\n", id.capacity.separated_string());
	print!("          ({}, {})\n",
		match decimal_prefix(id.capacity as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
		match binary_prefix(id.capacity as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
	);
	print!("Sector size (logical):  {}\n", id.sector_size_log);
	print!("Sector size (physical): {}\n", id.sector_size_phy);

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

fn print_attributes(values: &Vec<attr::SmartAttribute>) {
	print!("S.M.A.R.T. attribute values:\n");
	print!(" ID flags        value worst thresh raw\n");
	for val in values {
		print!("{:3} {}{}{}{}{}{} {:04x}    {:3}   {:3}    {} {:?}\n",
			val.id,
			bool_to_flag(val.pre_fail, 'P'),
			bool_to_flag(!val.online, 'O'),
			bool_to_flag(val.performance, 'S'),
			bool_to_flag(val.error_rate, 'R'),
			bool_to_flag(val.event_count, 'C'),
			bool_to_flag(val.self_preserving, 'K'),
			val.flags,
			val.value, val.worst,
			match val.thresh {
				Some(t) => format!("{:3}", t),
				None => "(?)".to_string(),
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
	print!("    │└───── O updated during off-line testing\n");
	print!("    └────── P prefailure warning\n");
}

// this also helps maintaining serialized output (JSON) clean
fn warn(msg: String) {
	// XXX unused result
	let _ = std::io::stderr().write(msg.as_bytes());
}

// XXX macro?
fn when_smart_enabled<F>(status: &id::Ternary, action_name: &str, mut action: F) where F: FnMut() -> () {
	match *status {
		id::Ternary::Unsupported => warn(format!("S.M.A.R.T. is not supported, cannot show {}\n", action_name)),
		id::Ternary::Disabled => warn(format!("S.M.A.R.T. is disabled, cannot show {}\n", action_name)),
		id::Ternary::Enabled => action(),
	}
}

fn main() {
	let args = App::new("smart-rs")
		.about("yet another S.M.A.R.T. querying tool")
		.version(crate_version!())
		.arg(Arg::with_name("health")
			.short("H") // smartctl-like
			.long("health") // smartctl-like
			.help("Prints the health status of the device")
		)
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
			.help("equivalent to -iHA")
		)
		.arg(Arg::with_name("json")
			.long("json")
			.help("Export data in JSON")
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
	let print_health = args.is_present("health") || args.is_present("all");

	let use_json = args.is_present("json");
	let mut json_map = serde_json::Map::new();

	if print_info || print_attrs || print_health {
		let data = ata::ata_exec(&file, ata::WIN_IDENTIFY, 1, 0, 1).unwrap();
		let id = id::parse_id(&data);

		if print_info {
			if use_json {
				json_map.insert("info".to_string(), id.to_json().unwrap());
			} else {
				print_id(&id);
			}
		}

		if print_health {
			when_smart_enabled(&id.smart, "health status", || {
				let data = ata::ata_task(&file,
					ata::SMART_CMD, ata::SMART_STATUS,
					0, 0, 0x4f, 0xc2, 0,
				).unwrap();
				let status = health::parse_smart_status(&data);

				if use_json {
					json_map.insert("health".to_string(), status.to_json().unwrap());
				} else {
					print!("S.M.A.R.T. health status: {}\n", match status {
						Some(true) => "good",
						Some(false) => "BAD",
						None => "(unknown)",
					});
				}
			});
		}

		if print_attrs {
			when_smart_enabled(&id.smart, "attributes", || {
				let data = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_VALUES, 1).unwrap();
				let thresh = ata::ata_exec(&file, ata::WIN_SMART, 0, ata::SMART_READ_THRESHOLDS, 1).unwrap();

				let values = attr::parse_smart_values(&data, &thresh);

				// TODO attribute names
				// TODO when-failed (now/past/never)

				if use_json {
					json_map.insert("attributes".to_string(), values.to_json().unwrap());
				} else {
					print_attributes(&values);
				}
			});
		}

		if use_json {
			print!("{}\n", serde_json::to_string(&json_map).unwrap());
		}
	}
}
