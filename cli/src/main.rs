extern crate hdd;

use hdd::ata;
use hdd::{Device, Direction};
use hdd::scsi::SCSIDevice;
use hdd::ata::ATADevice;

use hdd::ata::data::id;
use hdd::drivedb;

#[macro_use]
extern crate clap;
use clap::{
	App,
	Arg,
	SubCommand,
	AppSettings,
};

extern crate serde_json;
extern crate separator;
extern crate number_prefix;

mod info;
mod health;
mod attrs;

pub fn when_smart_enabled<F>(status: &id::Ternary, action_name: &str, mut action: F) where F: FnMut() -> () {
	match *status {
		id::Ternary::Unsupported => eprint!("S.M.A.R.T. is not supported, cannot show {}\n", action_name),
		id::Ternary::Disabled => eprint!("S.M.A.R.T. is disabled, cannot show {}\n", action_name),
		id::Ternary::Enabled => action(),
	}
}

type F = fn(&Device, Direction, &ata::RegistersWrite)
	-> Result<
		(ata::RegistersRead, Vec<u8>),
		std::io::Error
	>;

pub fn open_drivedb(option: Option<&str>) -> Option<Vec<drivedb::Entry>> {
	let drivedb = match option {
		Some(file) => drivedb::load(file).ok(), // .ok(): see below
		None => [
				"/var/lib/smartmontools/drivedb/drivedb.h",
				"/usr/local/share/smartmontools/drivedb.h", // for all FreeBSD folks out there
				"/usr/share/smartmontools/drivedb.h",
			].iter()
			.map(|f| drivedb::load(f).ok()) // .ok(): what's the point in collecting all these "no such file or directory" errors?
			.find(|ref db| db.is_some())
			.unwrap_or(None)
	};
	if drivedb.is_none() {
		eprint!("Cannot open drivedb file\n");
	};
	drivedb
}

pub fn get_device_id(ata_do: &F, dev: &Device) -> id::Id {
	let (_, data) = ata_do(&dev, Direction::From, &ata::RegistersWrite {
		command: ata::Command::Identify as u8,
		sector: 1,
		features: 0,
		sector_count: 1,
		cyl_high: 0,
		cyl_low: 0,
		device: 0,
	}).unwrap();
	id::parse_id(&data)
}

#[inline]
#[cfg(target_os = "linux")]
fn types() -> [&'static str; 1] { ["sat"] }
#[inline]
#[cfg(target_os = "freebsd")]
fn types() -> [&'static str; 2] { ["ata", "sat"] }

fn main() {
	let arg_json = Arg::with_name("json")
		.long("json")
		.help("Export data in JSON");
	let arg_drivedb = Arg::with_name("drivedb")
			.short("B") // smartctl-like
			.long("drivedb") // smartctl-like
			.takes_value(true)
			.value_name("FILE")
			.help("path to drivedb file"); // unlike smartctl, does not support '+FILE'

	let args = App::new("hdd")
		.about("yet another S.M.A.R.T. querying tool")
		.version(crate_version!())
		.setting(AppSettings::SubcommandRequired)
		.subcommand(SubCommand::with_name("health")
			.about("Prints the health status of the device")
			.arg(&arg_json)
		)
		.subcommand(SubCommand::with_name("info")
			.about("Prints a basic information about the device")
			.arg(&arg_json)
			.arg(&arg_drivedb)
		)
		.subcommand(SubCommand::with_name("attrs")
			.about("Prints a list of S.M.A.R.T. attributes")
			.arg(&arg_json)
			.arg(&arg_drivedb)
			.arg(Arg::with_name("vendorattribute")
				.multiple(true)
				.short("v") // smartctl-like
				.long("vendorattribute") // smartctl-like
				.takes_value(true)
				.value_name("id,format[:byteorder][,name]")
				.help("set display option for vendor attribute 'id'")
			)
		)
		.arg(Arg::with_name("type")
			.short("d") // smartctl-like
			.long("device") // smartctl-like
			.takes_value(true)
			.possible_values(&types())
			.help("device type")
		)
		.arg(Arg::with_name("device")
			.help("Device to query")
			.required(true)
			.index(1)
		)
		.get_matches();

	let satl: F = |dev: &Device, dir: Direction, regs: &ata::RegistersWrite| dev.ata_pass_through_16(dir, regs);
	let direct: F = |dev: &Device, dir: Direction, regs: &ata::RegistersWrite| dev.ata_do(dir, regs);
	let ata_do: F = match args.value_of("type") {
		Some("ata") if cfg!(target_os = "linux") => unreachable!(),
		Some("ata") if cfg!(target_os = "freebsd") => direct,
		Some("sat") => satl,
		// defaults
		None if cfg!(target_os = "linux") => satl,
		None if cfg!(target_os = "freebsd") => direct,
		_ => unreachable!(),
	};

	let dev = Device::open(
		args.value_of("device").unwrap()
	).unwrap();

	match args.subcommand() {
		("info", Some(args)) => info::info(&dev, &ata_do, &args),
		("health", Some(args)) => health::health(&dev, &ata_do, &args),
		("attrs", Some(args)) => attrs::attrs(&dev, &ata_do, &args),
		_ => unreachable!(),
	}
}
