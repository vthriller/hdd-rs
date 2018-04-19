use hdd::ata::data::id;
use hdd::drivedb;
use hdd::scsi::SCSICommon;
use hdd::scsi::data::inquiry;

use clap::{
	ArgMatches,
	App,
	SubCommand,
};

use serde_json;
use serde_json::value::ToJson;

use separator::Separatable;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

use ::{DeviceArgument, open_drivedb, arg_json, arg_drivedb};

fn bool_to_sup(b: bool) -> &'static str {
	if b { "supported" }
	else { "not supported" }
}

fn print_ata_id(id: &id::Id, meta: &Option<drivedb::DriveMeta>) {
	if id.incomplete { print!("WARNING: device reports information it provides is incomplete\n\n"); }

	// XXX id.is_ata is deemed redundant and is skipped
	// XXX we're skipping id.commands_supported for now as it is hardly of any interest to users

	print!("Model:    {}\n", id.model);
	match id.rpm {
		id::RPM::Unknown => (),
		id::RPM::NonRotating => print!("RPM:      N/A (SSD or other non-rotating media)\n"),
		id::RPM::RPM(i) => print!("RPM:      {}\n", i),
	};
	print!("Firmware: {}\n", id.firmware);
	print!("Serial:   {}\n", id.serial);
	// TODO: id.wwn_supported is cool, but actual WWN ID is better

	if let Some(ref meta) = *meta {
		if let Some(family) = meta.family {
			print!("Model family according to drive database:\n  {}\n", family);
		} else {
			print!("This drive is not in the drive database\n");
		}
		if let Some(warning) = meta.warning {
			print!("\n══════ WARNING ══════\n{}\n═════════════════════\n", warning);
		}
	}

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

fn print_scsi_id(inquiry: &inquiry::Inquiry) {
	print!("Vendor:   {}\n", inquiry.vendor_id);
	print!("Model:    {}\n", inquiry.product_id);
	print!("Firmware: {}\n", inquiry.product_rev);

	// TODO other inquiry fields, capacity, …
}

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("info")
		.about("Prints a basic information about the device")
		.arg(arg_json())
		.arg(arg_drivedb())
}

pub fn info(
	_: &Option<&str>,
	dev: &Option<&DeviceArgument>,
	args: &ArgMatches,
) {
	let dev = dev.unwrap_or_else(|| {
		// TODO show usage and whatnot
		eprint!("<device> is required\n");
		::std::process::exit(1);
	});

	let ata_id = match *dev {
		#[cfg(not(target_os = "linux"))]
		DeviceArgument::ATA(_, ref id) => Some(id),
		DeviceArgument::SAT(_, ref id) => Some(id),
		DeviceArgument::SCSI(_) => None,
	};

	let use_json = args.is_present("json");

	if let DeviceArgument::SCSI(ref dev) = *dev {
		let (_sense, data) = dev.scsi_inquiry(false, 0).unwrap();
		let inquiry = inquiry::parse_inquiry(&data);

		if use_json {
			let info = inquiry.to_json().unwrap();
			print!("{}\n", serde_json::to_string(&info).unwrap());
		} else {
			print_scsi_id(&inquiry);
		}
	}

	if let Some(id) = ata_id {
		let drivedb = open_drivedb(args.values_of("drivedb"));
		let meta = drivedb.as_ref().map(|drivedb| drivedb::render_meta(
			&id,
			drivedb,
			// no need to parse custom vendor attributes,
			// we're only using drivedb for the family and the warning here
			vec![],
		));

		if use_json {
			let mut info = id.to_json().unwrap();

			if let Some(ref meta) = meta {
				if let Some(family) = meta.family {
					info.as_object_mut().unwrap().insert("family".to_string(), family.to_json().unwrap());
				}
				if let Some(warning) = meta.warning {
					info.as_object_mut().unwrap().insert("warning".to_string(), warning.to_json().unwrap());
				}
			}

			print!("{}\n", serde_json::to_string(&info).unwrap());
		} else {
			print_ata_id(&id, &meta);
		}
	}
}
