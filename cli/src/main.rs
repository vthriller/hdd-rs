#![cfg_attr(feature = "cargo-clippy", allow(print_with_newline))]

#![warn(
	missing_debug_implementations,
	// TODO?..
	//missing_docs,
	//missing_copy_implementations,
	trivial_casts,
	trivial_numeric_casts,
	unsafe_code,
	unstable_features,
	unused_import_braces,
	unused_qualifications,
)]

extern crate hdd;

use hdd::Device;
use hdd::scsi::SCSIDevice;
use hdd::ata::ATADevice;
use hdd::ata::misc::Misc;

use hdd::ata::data::id;
use hdd::drivedb;

#[macro_use]
extern crate clap;
use clap::{
	ArgMatches,
	App,
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

pub fn open_drivedb(option: Option<&str>) -> Option<Vec<drivedb::Entry>> {
	let drivedb = match option {
		Some(file) => drivedb::load(file).ok(), // .ok(): see below
		None => [
				"/var/lib/smartmontools/drivedb/drivedb.h",
				"/usr/local/share/smartmontools/drivedb.h", // for all FreeBSD folks out there
				"/usr/share/smartmontools/drivedb.h",
			].iter()
			.map(|f| drivedb::load(f).ok()) // .ok(): what's the point in collecting all these "no such file or directory" errors?
			.find(|db| db.is_some())
			.unwrap_or(None)
	};
	if drivedb.is_none() {
		eprint!("Cannot open drivedb file\n");
	};
	drivedb
}

#[inline]
#[cfg(target_os = "linux")]
fn types() -> [&'static str; 1] { ["sat"] }
#[inline]
#[cfg(target_os = "freebsd")]
fn types() -> [&'static str; 2] { ["ata", "sat"] }

enum Type { ATA, SCSI }

type Arg = clap::Arg<'static, 'static>;
pub fn arg_json() -> Arg {
	Arg::with_name("json")
		.long("json")
		.help("Export data in JSON")
}
pub fn arg_drivedb() -> Arg {
	Arg::with_name("drivedb")
			.short("B") // smartctl-like
			.long("drivedb") // smartctl-like
			.takes_value(true)
			.value_name("FILE")
			.help("path to drivedb file") // unlike smartctl, does not support '+FILE'
}

type F<T: Misc + ?Sized> = fn(&str, &T, &ArgMatches);

fn main() {
	let args = App::new("hdd")
		.about("yet another disk querying tool")
		.version(crate_version!())
		.setting(AppSettings::SubcommandRequired)
		.subcommand(health::subcommand())
		.subcommand(info::subcommand())
		.subcommand(attrs::subcommand())
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

	let path = args.value_of("device").unwrap();
	let dev = Device::open(path).unwrap();

	let dtype = match args.value_of("type") {
		Some("ata") if cfg!(target_os = "linux") => unreachable!(),
		Some("ata") if cfg!(target_os = "freebsd") => Type::ATA,
		Some("sat") => Type::SCSI,
		// defaults
		None if cfg!(target_os = "linux") => Type::SCSI,
		None if cfg!(target_os = "freebsd") => Type::ATA,
		_ => unreachable!(),
	};

	// cannot have single `subcommand`: it must be of type `F<_>`, and you can't call `F<A>` and pass it `dev as &B` then
	let (subcommand_ata, subcommand_scsi, sargs): (F<ATADevice>, F<SCSIDevice>, _) = match args.subcommand() {
		("info", Some(args)) => (info::info, info::info, args),
		("health", Some(args)) => (health::health, health::health, args),
		("attrs", Some(args)) => (attrs::attrs, attrs::attrs, args),
		_ => unreachable!(),
	};

	match dtype {
		Type::ATA => subcommand_ata(path, &dev, sargs),
		Type::SCSI => subcommand_scsi(path, &SCSIDevice::new(dev), sargs),
	};
}
