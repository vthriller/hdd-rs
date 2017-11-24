use hdd::ata::misc::Misc;

use hdd::ata::data::attr;
use hdd::ata::data::attr::raw::Raw;
use hdd::drivedb;
use hdd::drivedb::vendor_attribute;

use hdd::scsi::pages::{Pages, ErrorCounter};

use clap::{
	Arg,
	ArgMatches,
	App,
	SubCommand,
};

use serde_json;
use serde_json::value::ToJson;

use std::collections::HashMap;
use std::string::ToString;

use std::f64::NAN;

use super::{DeviceArgument, open_drivedb, arg_drivedb};

fn bool_to_flag(b: bool, c: char) -> char {
	if b { c } else { '-' }
}

// XXX only `pretty_attributes` clearly shows failing/failed attributes
fn print_attributes(values: Vec<attr::SmartAttribute>) {
	if values.is_empty() {
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
	print!("                             ││││││\n");
	print!("                             │││││K auto-keep\n");
	print!("                             ││││C event count\n");
	print!("                             │││R error rate\n");
	print!("                             ││S speed/performance\n");
	print!("                             │O updated during off-line testing\n");
	print!("                             P prefailure warning\n");
}

fn escape(s: &String) -> String {
	s.chars()
		.flat_map(|c| c.escape_default())
		.collect()
}

fn format_prom<T: ToString>(key: &str, labels: &HashMap<&str, String>, value: T) -> String {
	let mut line = String::from(key);

	if ! labels.is_empty() {
		line.push('{');
		let mut labels = labels.into_iter();
		if let Some((k, v)) = labels.next() {
			line.push_str(&format!("{}=\"{}\"", k, escape(v)));
		}
		for (k, v) in labels {
			line.push_str(", ");
			line.push_str(&format!("{}=\"{}\"", k, escape(v)));
		}
		line.push('}');
	};
	line.push(' ');
	line.push_str(&value.to_string());
	line
}

fn print_prometheus_values(labels: &HashMap<&str, String>, values: Vec<attr::SmartAttribute>) {
	for val in values {
		let mut labels = labels.clone();
		labels.insert("id", val.id.to_string());
		labels.insert("name", val.name.unwrap_or("?".to_string()));
		labels.insert("pre_fail", val.pre_fail.to_string());

		val.value.map(|v| print!("{}\n", format_prom("smart_value", &labels, v)));
		val.worst.map(|v| print!("{}\n", format_prom("smart_worst", &labels, v)));
		val.thresh.map(|v| print!("{}\n", format_prom("smart_thresh", &labels, v)));
		print!("{}\n", format_prom("smart_raw", &labels, {
			use self::Raw::*;
			match val.raw {
				// TODO what should we do with these vecs from Raw{8,16}?
				Raw8(_) => NAN,
				Raw16(_) => NAN,
				Raw64(x) => x as f64,
				// TODO show opt value somehow?
				Raw16opt16(x, _) => x as f64,
				Raw16avg16 { value, .. } => value as f64,
				Raw24opt8(x, _) => x as f64,
				// TODO show div value somehow?
				Raw24div(x, _) => x as f64,
				Minutes(x) => x as f64,
				Seconds(x) => x as f64,
				HoursMilliseconds(h, ms) => (h as f64) * 3600. + (ms as f64) / 1000.,
				Celsius(x) => x as f64,
				// if you're exporting this into your monitoring system you already do not care about min and max that this drive reports
				CelsiusMinMax { current, .. } => current as f64,
			}
		}));
	}
}

pub fn subcommand() -> App<'static, 'static> {
	SubCommand::with_name("attrs")
		.about("Prints a list of S.M.A.R.T. attributes")
		.arg(Arg::with_name("format")
			.long("format")
			.takes_value(true)
			.possible_values(&["plain", "json", "prometheus"])
			.help("format to export data in")
		)
		.arg(Arg::with_name("json")
			.long("json")
			// for consistency with other subcommands
			.help("alias for --format=json")
			.overrides_with("format")
		)
		.arg(arg_drivedb())
		.arg(Arg::with_name("vendorattribute")
			.multiple(true)
			.short("v") // smartctl-like
			.long("vendorattribute") // smartctl-like
			.takes_value(true)
			.value_name("id,format[:byteorder][,name]")
			.help("set display option for vendor attribute 'id'")
		)
}

enum Format { Plain, JSON, Prometheus }
use self::Format::*;

fn attrs_ata(path: &str, dev: &DeviceArgument, format: Format, args: &ArgMatches) {
	let id = match *dev {
		DeviceArgument::ATA(ref dev) => dev.get_device_id().unwrap(),
		DeviceArgument::SAT(ref dev) => dev.get_device_id().unwrap(),
		DeviceArgument::SCSI(_) => unreachable!(),
	};

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
		drivedb,
		user_attributes,
	));

	// for --format=prometheus (TODO? don't compose if other format is used)
	let mut labels = HashMap::new();
	labels.insert("dev", path.to_string());
	labels.insert("model", id.model.clone());
	labels.insert("serial", id.serial.clone());
	if let Some(ref entry) = dbentry {
		if let Some(family) = entry.family {
			labels.insert("family", family.clone());
		}
	};

	use id::Ternary::*;
	match (format, id.smart) {
		(Plain, Unsupported) | (JSON, Unsupported) =>
			eprint!("S.M.A.R.T. is not supported, cannot show attributes\n"),
		(Prometheus, Unsupported) =>
			print!("{}\n", format_prom("smart_enabled", &labels, NAN)),

		(Plain, Disabled) | (JSON, Disabled) =>
			eprint!("S.M.A.R.T. is disabled, cannot show attributes\n"),
		(Prometheus, Disabled) =>
			print!("{}\n", format_prom("smart_enabled", &labels, 0)),

		(format, Enabled) => {
			let values = match *dev {
				DeviceArgument::ATA(ref dev) => dev.get_smart_attributes(&dbentry).unwrap(),
				DeviceArgument::SAT(ref dev) => dev.get_smart_attributes(&dbentry).unwrap(),
				DeviceArgument::SCSI(_) => unreachable!(),
			};

			match format {
				Plain => print_attributes(values),
				JSON => print!("{}\n",
					serde_json::to_string(
						&values.to_json().unwrap()
					).unwrap()
				),
				Prometheus => {
					print!("{}\n", format_prom("smart_enabled", &labels, 1));
					print_prometheus_values(&labels, values);
				},
			}
		},
	}
}

fn print_prom_scsi_error_counters(counters: &HashMap<ErrorCounter, u64>, action: &str) {
	let mut labels = HashMap::new();
	labels.insert("action", action.to_string());

	use self::ErrorCounter::*;
	for (k, v) in counters {
		match *k {
			CorrectedNoDelay => {
				let mut labels = labels.clone();
				labels.insert("with_delay", "1".to_string());
				print!("{}\n", format_prom("scsi_crc_corrected", &labels, v));
			},
			CorrectedDelay => {
				let mut labels = labels.clone();
				labels.insert("with_delay", "0".to_string());
				print!("{}\n", format_prom("scsi_crc_corrected", &labels, v));
			},

			ErrorsCorrected => {
				let mut labels = labels.clone();
				labels.insert("corrected", "1".to_string());
				print!("{}\n", format_prom("scsi_total_errors", &labels, v));
			},
			Uncorrected => {
				let mut labels = labels.clone();
				labels.insert("corrected", "0".to_string());
				print!("{}\n", format_prom("scsi_total_errors", &labels, v));
			},

			Total => { // XXX better name for this enum variant
				print!("{}\n", format_prom("scsi_repeated_actions", &labels, v));
			},
			CRCProcessed => {
				print!("{}\n", format_prom("scsi_crc_invocations", &labels, v));
			},
			BytesProcessed => {
				print!("{}\n", format_prom("scsi_bytes_processed", &labels, v));
			},

			VendorSpecific(n) | Reserved(n) => {
				let mut labels = labels.clone();
				labels.insert("id", format!("{}", n));
				print!("{}\n", format_prom("scsi_unknown_error_counter", &labels, v));
			},
		}
	}
}

// FIXME nice table formatting; for now, use `| column -ts$'\t'`
fn print_human_scsi_error_counters(counters: &Vec<(&str, HashMap<ErrorCounter, u64>)>) {
	use self::ErrorCounter::*;

	// header
	print!(".");
	for &(action, _) in counters.iter() {
		print!("\t{}", action);
	}
	print!("\n");

	let fixed = vec![
		(CorrectedNoDelay, "CRC corrected (instant)"),
		(CorrectedDelay, "CRC corrected (delayed)"),
		(Total, "Corrected (rereads, rewrites)"),
		(ErrorsCorrected, "Total errors (corrected)"),
		(Uncorrected, "Total errors (uncorrected)"),
		(CRCProcessed, "Total CRC invocations"),
		(BytesProcessed, "Bytes processed"),
	];

	for (key, name) in fixed {
		print!("{}", name);
		for &(_, ref values) in counters.iter() {
			print!("\t{}", values.get(&key)
				.map_or(
					"-".to_string(),
					|v| format!("{}", v),
				)
			);
		}
		print!("\n");
	}
}

// TODO other formats
// TODO prometheus: device id labels, just like in attrs_ata
fn attrs_scsi(path: &str, dev: &DeviceArgument, format: Format, args: &ArgMatches) {
	let dev = match *dev {
		DeviceArgument::ATA(_) | DeviceArgument::SAT(_) => unreachable!(),
		DeviceArgument::SCSI(ref dev) => dev,
	};


	let pages = match dev.supported_pages() {
		Ok(pages) => pages,
		Err(_) => return, // TODO
	};

	// XXX should check if page is supported in `trait Pages` methods themselves, not here

	// TODO Err() returned by dev.*_error_counters()
	let cnt_err_write    = if pages.contains(&0x02) { dev.write_error_counters().ok()        } else { None };
	let cnt_err_read     = if pages.contains(&0x03) { dev.read_error_counters().ok()         } else { None };
	let cnt_err_read_rev = if pages.contains(&0x04) { dev.read_reverse_error_counters().ok() } else { None };
	let cnt_err_verify   = if pages.contains(&0x05) { dev.verify_error_counters().ok()       } else { None };

	match format {
		Prometheus => {
			cnt_err_write.map(|counters| print_prom_scsi_error_counters(&counters, "write"));
			cnt_err_read.map(|counters| print_prom_scsi_error_counters(&counters, "read"));
			cnt_err_read_rev.map(|counters| print_prom_scsi_error_counters(&counters, "read-reverse"));
			cnt_err_verify.map(|counters| print_prom_scsi_error_counters(&counters, "verify"));
		},
		Plain => {
			let mut table = vec![];
			cnt_err_write.map(|counters| table.push(("write", counters)));
			cnt_err_read.map(|counters| table.push(("read", counters)));
			cnt_err_read_rev.map(|counters| table.push(("read-reverse", counters)));
			cnt_err_verify.map(|counters| table.push(("verify", counters)));

			print_human_scsi_error_counters(&table);
		},
		_ => unimplemented!(),
	}
}

pub fn attrs(
	path: &str,
	dev: &DeviceArgument,
	args: &ArgMatches,
) {
	let format = match args.value_of("format") {
		Some("plain") => Plain,
		Some("json") => JSON,
		Some("prometheus") => Prometheus,
		None if args.is_present("json") => JSON,
		None => Plain,
		_ => unreachable!(),
	};

	use DeviceArgument::*;
	match dev {
		dev @ &ATA(_) | dev @ &SAT(_) => attrs_ata(path, dev, format, args),
		dev @ &SCSI(_) => attrs_scsi(path, dev, format, args),
	};
}
