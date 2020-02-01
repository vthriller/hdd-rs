use hdd::ata::misc::Misc;

use hdd::ata::data::attr::{self, SmartAttributes};
use hdd::ata::data::attr::raw::Raw;
use hdd::drivedb;
use hdd::drivedb::vendor_attribute;

use hdd::scsi::pages::{SCSIPages, ErrorCounter};
use hdd::scsi::{SCSICommon, DefectList};
use hdd::scsi::data::inquiry;

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
use std::path::Path;

use std::f64::NAN;

use number_prefix::{decimal_prefix, binary_prefix, Prefixed, Standalone};

use prettytable;
use prettytable::Table;
use prettytable::row::Row;
use prettytable::cell::Cell;

use ::{DeviceArgument, open_drivedb};
use super::{Subcommand, arg_drivedb};

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
		let value = val.value();
		let worst = val.worst();
		let thresh = val.thresh();
		// > The NAME … should not exceed 23 characters
		print!("{:3} {:.<24} {}{}{}{}{}{}{}    {}   {}    {} {} {}\n",
			val.id,
			val.name().unwrap_or(&"?".to_string()),
			bool_to_flag(val.pre_fail(), 'P'),
			bool_to_flag(!val.online(), 'O'),
			bool_to_flag(val.performance(), 'S'),
			bool_to_flag(val.error_rate(), 'R'),
			bool_to_flag(val.event_count(), 'C'),
			bool_to_flag(val.self_preserving(), 'K'),
			if val.misc_flags() == 0 { "     ".to_string() }
				else { format!("+{:04x}", val.misc_flags()) },
			value.map(|v| format!("{:3}", v)).unwrap_or("---".to_string()),
			worst.map(|v| format!("{:3}", v)).unwrap_or("---".to_string()),
			thresh.map(|v| format!("{:3}", v)).unwrap_or("(?)".to_string()),
			match (value, worst, thresh) {
				(Some(v), _, Some(t)) if v <= t => "NOW ",
				(_, Some(w), Some(t)) if w <= t => "past",
				// either value/worst are part of the `val.row`,
				// or threshold is not available,
				// or value never was below the threshold
				_ => "-   ",
			},
			val.raw(),
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
		labels.insert("name", val.name().unwrap_or(&"?".to_string()).to_string());
		labels.insert("pre_fail", val.pre_fail().to_string());

		val.value().map(|v| print!("{}\n", format_prom("smart_value", &labels, v)));
		val.worst().map(|v| print!("{}\n", format_prom("smart_worst", &labels, v)));
		val.thresh().map(|v| print!("{}\n", format_prom("smart_thresh", &labels, v)));
		print!("{}\n", format_prom("smart_raw", &labels, {
			use self::Raw::*;
			match val.raw() {
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

pub struct Attrs {}
impl Subcommand for Attrs {
	fn subcommand(&self) -> App<'static, 'static> {
		SubCommand::with_name("attrs")
			.about("Prints a list of S.M.A.R.T. attributes")
			.arg(Arg::with_name("format")
				.long("format")
				.takes_value(true)
				.possible_values(&["plain", "json", "prometheus"])
				.help("format to export data in")
			)
			.arg(Arg::with_name("full-path")
				.long("full-path")
				.help("whether to use full device path if exporting in prometheus format")
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

	fn run(
		&self,
		path: &Option<&Path>,
		dev: &Option<&DeviceArgument>,
		args: &ArgMatches,
	) {
		let dev = dev.unwrap_or_else(|| {
			// TODO show usage and whatnot
			eprint!("<device> is required\n");
			::std::process::exit(1);
		});
		let path = path.unwrap(); // `path` and `dev` are both `Some()` or both `None`

		let path = if args.is_present("full-path") {
			path.to_str()
		} else {
			path.file_name().unwrap().to_str()
		}.unwrap();

		let format = match args.value_of("format") {
			Some("plain") => Plain,
			Some("json") => JSON,
			Some("prometheus") => Prometheus,
			None if args.is_present("json") => JSON,
			None => Plain,
			_ => unreachable!(),
		};

		let user_attributes = args.values_of("vendorattribute")
			.map(|attrs| attrs.collect())
			.unwrap_or(vec![])
			.into_iter()
			.map(|attr| vendor_attribute::parse(attr).ok()) // TODO Err(_)
			.filter(|x| x.is_some())
			.map(|x| x.unwrap())
			.collect();
		let drivedb = open_drivedb(args.values_of("drivedb"));

		use DeviceArgument::*;
		match dev {
			#[cfg(not(target_os = "linux"))]
			dev @ ATA(_, _) => attrs_ata(path, dev, format, drivedb, user_attributes),
			dev @ SAT(_, _) => attrs_ata(path, dev, format, drivedb, user_attributes),
			dev @ SCSI(_) => attrs_scsi(path, dev, format),
		};
	}
}

#[derive(PartialEq)]
enum Format { Plain, JSON, Prometheus }
use self::Format::*;

fn attrs_ata(path: &str, dev: &DeviceArgument, format: Format, drivedb: Option<drivedb::DriveDB>, user_attributes: Vec<drivedb::Attribute>) {
	let id = match dev {
		#[cfg(not(target_os = "linux"))]
		DeviceArgument::ATA(_, id) => id,
		DeviceArgument::SAT(_, id) => id,
		DeviceArgument::SCSI(_) => unreachable!(),
	};

	let dbentry = drivedb.as_ref().map(|drivedb| drivedb.render_meta(
		&id,
		&user_attributes,
	));

	// for --format=prometheus (TODO? don't compose if other format is used)
	let mut labels = HashMap::new();
	labels.insert("dev", path.to_string());
	labels.insert("model", id.model.clone());
	labels.insert("serial", id.serial.clone());
	if let Some(entry) = &dbentry {
		if let Some(family) = entry.family {
			labels.insert("family", family.clone());
		}
	};

	use id::Ternary::*;

	match (&format, &dbentry) {
		(Prometheus, Some(ref entry)) => {
			if let Some(_) = entry.warning {
				print!("{}\n", format_prom("smart_drivedb_warning", &labels, 1));
			}
		},
		_ => (),
	}

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
			let mut values = match dev {
				#[cfg(not(target_os = "linux"))]
				DeviceArgument::ATA(dev, _) => dev.get_smart_attributes().unwrap(),
				DeviceArgument::SAT(dev, _) => dev.get_smart_attributes().unwrap(),
				DeviceArgument::SCSI(_) => unreachable!(),
			};

			values.annotate(&dbentry);

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

fn print_prom_scsi_error_counters(labels: &HashMap<&str, String>, counters: &HashMap<ErrorCounter, u64>, action: &str) {
	let mut labels = labels.clone();
	labels.insert("action", action.to_string());

	use self::ErrorCounter::*;
	for (k, v) in counters {
		match k {
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

fn scsi_error_counters_json(counters: &HashMap<ErrorCounter, u64>) -> serde_json::Value {
	let mut json = serde_json::Map::new();

	use self::ErrorCounter::*;
	for (&k, &v) in counters {
		let v = v.to_json().unwrap();
		match k {
			// TODO? submaps for CRC, totals

			CorrectedNoDelay => json.insert("crc-corrected-instant".to_string(), v),
			CorrectedDelay => json.insert("crc-corrected-delay".to_string(), v),

			ErrorsCorrected => json.insert("total-corrected".to_string(), v),
			Uncorrected => json.insert("total-uncorrected".to_string(), v),

			Total => json.insert("corrected-repeated-actions".to_string(), v),
			CRCProcessed => json.insert("crc-processed".to_string(), v),
			BytesProcessed => json.insert("bytes-processed".to_string(), v),

			VendorSpecific(n) => json.insert(format!("vendor-specific-{}", n), v),
			Reserved(n) => json.insert(format!("reserved-{}", n), v),
		};
	}

	json.to_json().unwrap()
}

fn print_human_scsi_error_counters(counters: &Vec<(&str, HashMap<ErrorCounter, u64>)>) {
	use self::ErrorCounter::*;

	// no columns to show?
	if counters.is_empty() { return; }

	let mut table = Table::new();
	table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

	// header
	{
		let mut row = vec![];

		row.push(Cell::new(""));

		for &(action, _) in counters.iter() {
			row.push(Cell::new(action).style_spec("r"));
		}

		table.set_titles(Row::new(row));
	}

	let mut rows = vec![
		(CorrectedNoDelay, "CRC corrected (instant)".to_string()),
		(CorrectedDelay, "CRC corrected (delayed)".to_string()),
		(Total, "Corrected (rereads, rewrites)".to_string()),
		(ErrorsCorrected, "Total errors (corrected)".to_string()),
		(Uncorrected, "Total errors (uncorrected)".to_string()),
		(CRCProcessed, "Total CRC invocations".to_string()),
		(BytesProcessed, "Bytes processed".to_string()),
	];

	// FIXME this whole thing about unexpected keys is fragile AF, mainly because of all the unreachable!()s

	let mut unexpected = vec![];
	for (_, values) in counters.iter() {
		for (key, _) in values {
			match *key {
				CorrectedNoDelay | CorrectedDelay | Total | ErrorsCorrected | Uncorrected | CRCProcessed | BytesProcessed
					=> (), // skip keys from the `rows` above
				key @ VendorSpecific(_) | key @ Reserved(_) =>
					unexpected.push(key),
			}
		}
	}
	unexpected.sort_unstable_by(|a, b| {
		use std::cmp::Ordering::*;
		match (a, b) {
			(VendorSpecific(a), VendorSpecific(b)) => a.cmp(&b),
			(      Reserved(a),       Reserved(b)) => a.cmp(&b),
			(VendorSpecific(_),       Reserved(_)) => Greater,
			(      Reserved(_), VendorSpecific(_)) => Less,
			_ => unreachable!(),
		}
	});
	unexpected.dedup();

	for key in unexpected {
		match key {
			VendorSpecific(n) => rows.push((key, format!("vendor-specific (0x{:02x})", n))),
			Reserved(n)       => rows.push((key, format!("reserved (0x{:02x})", n))),
			_ => unreachable!(),
		}
	}

	for (key, name) in rows {
		let mut row = vec![];

		row.push(Cell::new(&name));

		for (_, values) in counters.iter() {
			#[allow(trivial_casts)] // closures in .map_or()
			row.push(Cell::new(&values.get(&key)
				.map_or(
					"-".to_string(),
					if key == BytesProcessed {
						(|&v| match binary_prefix(v as f32) {
							Prefixed(p, x) => format!("{:.1} {}B", x, p),
							Standalone(x)  => format!("{} B", x),
						}) as fn(&u64) -> String
					} else {
						(|&v| match decimal_prefix(v as f32) {
							Prefixed(p, x) => format!("{:.1}{}", x, p),
							Standalone(x)  => format!("{}", x),
						}) as fn(&u64) -> String
					},
				)
			).style_spec("r"));
		}

		table.add_row(Row::new(row));
	}

	table.printstd();
}

// TODO other formats
// TODO prometheus: device id labels, just like in attrs_ata
fn attrs_scsi(path: &str, dev: &DeviceArgument, format: Format) {
	let dev = match dev {
		#[cfg(not(target_os = "linux"))]
		DeviceArgument::ATA(_, _) => unreachable!(),
		DeviceArgument::SAT(_, _) => unreachable!(),
		DeviceArgument::SCSI(dev) => dev,
	};

	let mut pages = SCSIPages::new(dev);
	if let Err(e) = &pages {
		eprint!("cannot access SCSI log pages: {}\n", e);
	}

	let mut json = serde_json::Map::new();

	let mut labels = HashMap::new();
	labels.insert("dev", path.to_string());
	if let Ok((_sense, data)) = dev.scsi_inquiry(false, 0) {
		let inquiry = inquiry::parse_inquiry(&data);
		labels.insert("vendor", inquiry.vendor_id.clone());
		labels.insert("model", inquiry.product_id.clone());
		labels.insert("firmware", inquiry.product_rev.clone());
	}

	if let Ok(ref mut pages) = pages {
		// TODO Err() returned by dev.*_error_counters()
		let error_counters = vec![
			("write",        pages.write_error_counters().ok()),
			("read",         pages.read_error_counters().ok()),
			("read-reverse", pages.read_reverse_error_counters().ok()),
			("verify",       pages.verify_error_counters().ok()),
		];

		match format {
			Prometheus => {
				for (name, counters) in error_counters {
					counters.map(|counters| print_prom_scsi_error_counters(&labels, &counters, name));
				}
			},
			Plain => {
				let mut table = vec![];
				for (name, counters) in error_counters {
					counters.map(|counters| table.push((name, counters)));
				}
				print_human_scsi_error_counters(&table);
			},
			JSON => {
				for (name, counters) in error_counters {
					if let Some(counters) = counters {
						json.insert(name.to_string(), scsi_error_counters_json(&counters));
					}
				}
			},
		}
	}

	// Non-medium errors

	// also TODO Err()
	if let Some(Ok(x)) = pages.iter_mut().next().map(|p| p.non_medium_error_count()) {
		match format {
			Prometheus => {
				print!("{}\n", format_prom("scsi_non_medium_errors", &labels, x));
			},
			Plain => {
				print!("\nNon-medium errors: {}\n", x);
			},
			JSON => {
				json.insert("non-medium-errors".to_string(), x.to_json().unwrap());
			},
		}
	}

	// Temperature

	// also TODO Err()
	if let Some(Ok((temp, ref_temp))) = pages.iter_mut().next().map(|p| p.temperature()) {
		match format {
			Prometheus => {
				if let Some(t) = temp     { print!("{}\n", format_prom("scsi_temperature", &labels, t)) };
				if let Some(t) = ref_temp { print!("{}\n", format_prom("scsi_reference_temperature", &labels, t)) };
			},
			Plain => {
				if let Some(t) = temp {
					print!("\nTemperature: {}°C", t);
					if let Some(t) = ref_temp {
						print!(" (max allowed: {}°C)", t);
					}
					print!("\n");
				}
			},
			JSON => {
				let mut tmp = serde_json::Map::new();
				tmp.insert("current".to_string(), temp.to_json().unwrap());
				tmp.insert("reference".to_string(), ref_temp.to_json().unwrap());
				json.insert("temperature".to_string(), tmp.to_json().unwrap());
			},
		}
	}

	// Start-Stop Cycle Counters

	// also TODO Err()
	// FIXME copy-paste: cycles.{,_lifetime}{start_stop,load_unload}_cycles
	if let Some(Ok(cycles)) = pages.iter_mut().next().map(|p| p.dates_and_cycle_counters()) {
		match format {
			Prometheus => {
				let mut labels = labels.clone();

				labels.insert("action", "start-stop".to_string());
				if let Some(t) = cycles.start_stop_cycles          { print!("{}\n", format_prom("scsi_cycles", &labels, t)) };
				if let Some(t) = cycles.lifetime_start_stop_cycles { print!("{}\n", format_prom("scsi_lifetime_cycles", &labels, t)) };

				labels.insert("action", "load-unload".to_string());
				if let Some(t) = cycles.load_unload_cycles          { print!("{}\n", format_prom("scsi_cycles", &labels, t)) };
				if let Some(t) = cycles.lifetime_load_unload_cycles { print!("{}\n", format_prom("scsi_lifetime_cycles", &labels, t)) };
			},
			Plain => {
				print!("\n");
				if let Some(x) = cycles.start_stop_cycles {
					print!("Start-stop cycles: {}", x);
					if let Some(x) = cycles.lifetime_start_stop_cycles {
						print!("/{}", x);
					}
					print!("\n");
				}
				if let Some(x) = cycles.load_unload_cycles {
					print!("Load-unload cycles: {}", x);
					if let Some(x) = cycles.lifetime_load_unload_cycles {
						print!("/{}", x);
					}
					print!("\n");
				}
			},
			JSON => {
				let mut tmp = serde_json::Map::new();

				let mut values = serde_json::Map::new();
				values.insert("current".to_string(), cycles.start_stop_cycles.to_json().unwrap());
				values.insert("lifetime".to_string(), cycles.lifetime_start_stop_cycles.to_json().unwrap());
				tmp.insert("start-stop".to_string(), values.to_json().unwrap());

				let mut values = serde_json::Map::new();
				values.insert("current".to_string(), cycles.load_unload_cycles.to_json().unwrap());
				values.insert("lifetime".to_string(), cycles.lifetime_load_unload_cycles.to_json().unwrap());
				tmp.insert("load-unload".to_string(), values.to_json().unwrap());

				json.insert("cycles".to_string(), tmp.to_json().unwrap());
			},
		}
	}

	// Grown Defect List

	// again, TODO Err()
	if let Ok(Some(defects)) = dev.read_defect_data_10(DefectList::Grown) {
		match format {
			Prometheus => {
				labels.insert("list", "grown".to_string());
				print!("{}\n", format_prom("scsi_defects", &labels, defects));
			},
			Plain => {
				print!("\n");
				print!("Elements in grown defect list: {}\n", defects);
			},
			JSON => {
				let mut tmp = serde_json::Map::new();
				tmp.insert("grown".to_string(), defects.to_json().unwrap());
				json.insert("defect-list".to_string(), tmp.to_json().unwrap());
			},
		}
	}

	if format == JSON {
		print!("{}\n", serde_json::to_string(&json).unwrap());
	}
}
