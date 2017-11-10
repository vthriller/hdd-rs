use hdd::ata::misc::Misc;

use clap::{
	App,
	ArgMatches,
	SubCommand,
};

use serde_json;
use serde_json::value::ToJson;

use super::{when_smart_enabled, arg_json};

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("health")
		.about("Prints the health status of the device")
		.arg(arg_json())
}

pub fn health<T: Misc + ?Sized>(
	dev: &T,
	args: &ArgMatches,
) {
	let id = dev.get_device_id().unwrap();

	let use_json = args.is_present("json");

	when_smart_enabled(&id.smart, "health status", || {
		let status = dev.get_smart_health().unwrap();

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
