use hdd::device::Device;

use clap::{
	ArgMatches,
	App,
	SubCommand,
};

use super::DeviceArgument;

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("list")
		.about("Lists disk devices")
}

pub fn list(
	_: &Option<&str>,
	dev: &Option<&DeviceArgument>,
	_: &ArgMatches,
) {
	if dev.is_some() {
		// TODO show usage and whatnot
		eprint!("<device> is redundant\n");
		::std::process::exit(1);
	};

	for dev in Device::list_devices() {
		print!("{}\n", dev.into_os_string().to_str().unwrap());
	}
}
