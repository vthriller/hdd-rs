use hdd;

use hdd::ata;
use hdd::Direction;

use hdd::ata::data::attr;
use hdd::drivedb;
use hdd::drivedb::vendor_attribute;

use clap::ArgMatches;

use serde_json;
use serde_json::value::ToJson;

use super::{F, get_device_id, open_drivedb, when_smart_enabled};

fn bool_to_flag(b: bool, c: char) -> char {
	if b { c } else { '-' }
}

// XXX only `pretty_attributes` clearly shows failing/failed attributes
fn print_attributes(values: &Vec<attr::SmartAttribute>) {
	if values.len() == 0 {
		print!("No S.M.A.R.T. attributes found.\n");
		return;
	}

	print!("S.M.A.R.T. attribute values:\n");
	print!(" ID name                     flags        value worst thresh fail raw\n");
	for val in values {
		// > The NAME … should not exceed 23 characters
		print!("{:3} {:.<24} {}{}{}{}{}{}{}    {}   {}    {} {} {}\n",
			val.id,
			val.name.as_ref().unwrap_or(&"?".to_string()),
			bool_to_flag(val.pre_fail, 'P'),
			bool_to_flag(!val.online, 'O'),
			bool_to_flag(val.performance, 'S'),
			bool_to_flag(val.error_rate, 'R'),
			bool_to_flag(val.event_count, 'C'),
			bool_to_flag(val.self_preserving, 'K'),
			if val.flags == 0 { "     ".to_string() }
				else { format!("+{:04x}", val.flags) },
			val.value.map(|v| format!("{:3}", v)).unwrap_or("---".to_string()),
			val.worst.map(|v| format!("{:3}", v)).unwrap_or("---".to_string()),
			val.thresh.map(|v| format!("{:3}", v)).unwrap_or("(?)".to_string()),
			match (val.value, val.worst, val.thresh) {
				(Some(v), _, Some(t)) if v <= t => "NOW ",
				(_, Some(w), Some(t)) if w <= t => "past",
				// either value/worst are part of the `val.row`,
				// or threshold is not available,
				// or value never was below the threshold
				_ => "-   ",
			},
			val.raw,
		);
	}
	// based on the output of 'smartctl -A -f brief' (part of 'smartctl -x')
	print!("                             │││││└─ K auto-keep\n");
	print!("                             ││││└── C event count\n");
	print!("                             │││└─── R error rate\n");
	print!("                             ││└──── S speed/performance\n");
	print!("                             │└───── O updated during off-line testing\n");
	print!("                             └────── P prefailure warning\n");
}

pub fn attrs(
	dev: &hdd::Device,
	ata_do: &F,
	args: &ArgMatches,
) {
	let id = get_device_id(ata_do, dev);

	let user_attributes = args.values_of("vendorattribute")
		.map(|attrs| attrs.collect())
		.unwrap_or(vec![])
		.into_iter()
		.map(|attr| vendor_attribute::parse(attr).ok()) // TODO Err(_)
		.filter(|x| x.is_some())
		.map(|x| x.unwrap())
		.collect();

	let drivedb = open_drivedb(args.value_of("drivedb"));
	let dbentry = drivedb.as_ref().map(|drivedb| drivedb::match_entry(
		&id,
		&drivedb,
		user_attributes,
	));

	let use_json = args.is_present("json");

	when_smart_enabled(&id.smart, "attributes", || {
		let (_, data) = ata_do(&dev, Direction::From, &ata::RegistersWrite {
			command: ata::Command::SMART as u8,
			sector: 0,
			features: ata::SMARTFeature::ReadValues as u8,
			sector_count: 1,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		}).unwrap();
		let (_, thresh) = ata_do(&dev, Direction::From, &ata::RegistersWrite {
			command: ata::Command::SMART as u8,
			sector: 0,
			features: ata::SMARTFeature::ReadThresholds as u8,
			sector_count: 1,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		}).unwrap();

		let values = attr::parse_smart_values(&data, &thresh, &dbentry);

		if use_json {
			print!("{}\n", serde_json::to_string(&values.to_json().unwrap()).unwrap());
		} else {
			print_attributes(&values);
		}
	});
}
