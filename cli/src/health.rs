use hdd::ata::misc::Misc;

use clap::{
	App,
	ArgMatches,
	SubCommand,
};

use serde_json;
use serde_json::value::ToJson;

use super::{DeviceArgument, when_smart_enabled, arg_json};

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("health")
		.about("Prints the health status of the device")
		.arg(arg_json())
}

pub fn health(
	_: &str,
	dev: &DeviceArgument,
	args: &ArgMatches,
) {
	let id = match *dev {
		DeviceArgument::ATA(ref dev) => dev.get_device_id().unwrap(),
		DeviceArgument::SAT(ref dev) => dev.get_device_id().unwrap(),
		DeviceArgument::SCSI(_) => unimplemented!(),
	};

	let use_json = args.is_present("json");

	when_smart_enabled(&id.smart, "health status", || {
		let status = match *dev {
			DeviceArgument::ATA(ref dev) => dev.get_smart_health().unwrap(),
			DeviceArgument::SAT(ref dev) => dev.get_smart_health().unwrap(),
			DeviceArgument::SCSI(_) => unimplemented!(),
		};

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
