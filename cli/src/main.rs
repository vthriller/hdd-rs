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

use hdd::{device, Device};
use hdd::scsi::SCSIDevice;
use hdd::ata::ATADevice;

use hdd::ata::data::id;
use hdd::drivedb;
use hdd::ata::misc::{self, Misc};
use hdd::scsi::ATAError;

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

#[cfg(target_os = "linux")]
arg_enum! {
	enum Type { Auto, SAT, SCSI }
}

#[cfg(target_os = "freebsd")]
arg_enum! {
	enum Type { Auto, ATA, SAT, SCSI }
}

#[derive(Debug)]
pub enum DeviceArgument {
	ATA(ATADevice<Device>, id::Id),
	SAT(ATADevice<SCSIDevice>, id::Id),
	SCSI(SCSIDevice),
}

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

type F = fn(&str, &DeviceArgument, &ArgMatches);

fn main() {
	/*
	XXX this bit of clap.rs lets me down
	we want to allow users to type in types in lower case, but .possible_values() would not allow that unless we pass it modified list of values
	so why do we do it here and not in-place?
	- to_ascii_lowercase() returns `String`s, but .possible_values() only accepts `&str`s, so someone needs to own them. Sigh.
	- the result looks somewhat clunky.

	see also https://github.com/kbknapp/clap-rs/issues/891
	*/
	let type_variants: Vec<_> = Type::variants().iter()
		.map(|s| std::ascii::AsciiExt::to_ascii_lowercase(s.to_owned()))
		.collect();
	// previous we'll never need original values, so shadow them with the references
	let type_variants: Vec<_> = type_variants.iter()
		.map(|s| &**s)
		.collect();

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
			.possible_values(type_variants.as_slice())
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

	let dtype = args.value_of("type")
		.unwrap_or("auto")
		.parse::<Type>().unwrap();

	#[allow(unused_variables)] // ignore subcommand_ata if cfg!(target_os = "linux")
	let (subcommand, sargs): (F, _) = match args.subcommand() {
		("info", Some(args)) => (info::info, args),
		("health", Some(args)) => (health::health, args),
		("attrs", Some(args)) => (attrs::attrs, args),
		_ => unreachable!(),
	};

	/*
	Why do we issue ATA IDENTIFY DEVICE here?
	- Device id is what every subcommand uses for one reason or the other, but usually to check whether some feature is supported and enabled.
	- It allows us to distinguish between pure SCSI devices and ATA devices behind SAT by issuing ATA PASS-THROUGH and checking whether this command is supported.
	*/

	let dev = match dtype {
		Type::Auto => {
			match dev.get_type().unwrap() {
				device::Type::SCSI => {
					// check whether devices replies to ATA PASS-THROUGH
					let satdev = ATADevice::new(SCSIDevice::new(dev));
					match satdev.get_device_id() {
						// this is really an ATA device
						Ok(id) =>
							DeviceArgument::SAT(satdev, id),
						// nnnnope, plain SCSI
						Err(misc::Error::SCSI(ATAError::NotSupported)) =>
							DeviceArgument::SCSI(satdev.unwrap()),
						// huh? time to contact Houston
						e => {
							e.unwrap(); // TODO abort gracefully
							unreachable!() // we already panicked
						},
					}
				},
				device::Type::ATA => {
					let atadev = ATADevice::new(dev);
					let id = atadev.get_device_id().unwrap();
					DeviceArgument::ATA(atadev, id)
				},
			}
		},
		#[cfg(target_os = "freebsd")]
		Type::ATA => {
			let dev = ATADevice::new(dev);
			let id = dev.get_device_id().unwrap();
			DeviceArgument::ATA(dev, id)
		},
		Type::SAT => {
			let dev = ATADevice::new(SCSIDevice::new(dev));
			let id = dev.get_device_id().unwrap();
			DeviceArgument::SAT(dev, id)
		},
		Type::SCSI => DeviceArgument::SCSI(SCSIDevice::new(dev)),
	};

	subcommand(path, &dev, sargs)
}
