use hdd;

use hdd::ata;
use hdd::Direction;

use hdd::ata::data::health;

use clap::{
	App,
	ArgMatches,
	SubCommand,
};

use serde_json;
use serde_json::value::ToJson;

use super::{F, get_device_id, when_smart_enabled, arg_json};

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("health")
		.about("Prints the health status of the device")
		.arg(arg_json())
}

pub fn health(
	dev: &hdd::Device,
	ata_do: &F,
	args: &ArgMatches,
) {
	let id = get_device_id(ata_do, dev);

	let use_json = args.is_present("json");

	when_smart_enabled(&id.smart, "health status", || {
		let (regs, _) = ata_do(&dev, Direction::None, &ata::RegistersWrite {
			command: ata::Command::SMART as u8,
			features: ata::SMARTFeature::ReturnStatus as u8,
			sector_count: 0,
			sector: 0,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		}).unwrap();
		let status = health::parse_smart_status(&regs);

		if use_json {
			print!("{}\n", serde_json::to_string(&status.to_json().unwrap()).unwrap());
		} else {
			print!("S.M.A.R.T. health status: {}\n", match status {
				Some(true) => "good",
				Some(false) => "BAD",
				None => "(unknown)",
			});
		}
	});
}
