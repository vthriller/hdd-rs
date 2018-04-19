use hdd::device::Device;

use clap::{
	ArgMatches,
	App,
	SubCommand,
};

use serde_json;

use ::{Subcommand, DeviceArgument, arg_json};

pub struct List {}
impl Subcommand for List {
fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("list")
		.about("Lists disk devices")
		.arg(arg_json())
}
}

pub fn list(
	_: &Option<&str>,
	dev: &Option<&DeviceArgument>,
	args: &ArgMatches,
) {
	if dev.is_some() {
		// TODO show usage and whatnot
		eprint!("<device> is redundant\n");
		::std::process::exit(1);
	};

	let devs = Device::list_devices().unwrap_or_else(|err| {
		eprint!("Cannot list devices: {}\n", err);
		::std::process::exit(1);
	});

	if args.is_present("json") {
		print!("{}\n", serde_json::to_string(&devs).unwrap());
	} else {
		for dev in devs {
			print!("{}\n", dev.into_os_string().to_str().unwrap());
		}
	}
}
