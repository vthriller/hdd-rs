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
use hdd::drivedb::{self, Entry};

#[macro_use]
extern crate clap;
use clap::{App, Arg};

use std::time::Instant;
use std::collections::HashSet;

extern crate regex;
use regex::bytes::{Regex, RegexSet};
extern crate regex_cache;
use regex_cache::LazyRegex;

// borrowed from https://crates.io/crates/criterion (Apache-2.0/MIT)
// cannot just write this crate in as a dependency due to some dep resolution conflicts
// see also https://github.com/rust-lang/rfcs/issues/1484
#[allow(unsafe_code)]
pub fn black_box<T>(dummy: T) -> T {
	unsafe {
		let ret = std::ptr::read_volatile(&dummy);
		std::mem::forget(dummy);
		ret
	}
}

fn elapsed(i: Instant) -> f32 {
	let elapsed = i.elapsed();
	(elapsed.as_secs() as f32 + elapsed.subsec_nanos() as f32/1e9) * 1e3
}

fn find<'a>(db: &'a Vec<Entry>, model: &str, firmware: &str) -> Option<&'a Entry> {
	for entry in db.iter() {
		if entry.model.starts_with("USB:") { continue }

		// model and firmware are expected to be ascii strings, no need to try matching unicode characters

		// > [modelregexp] should never be "".
		let re = Regex::new(format!("(?-u)^{}$", entry.model).as_str()).unwrap();
		if !re.is_match(model.as_bytes()) { continue }

		if ! entry.firmware.is_empty() {
			let re = Regex::new(format!("^(?-u){}$", entry.firmware).as_str()).unwrap();
			if !re.is_match(firmware.as_bytes()) { continue }
		}

		return Some(entry);
	}

	None
}

fn find_precompiled<'a>(db: &Vec<(&'a Entry, Regex, Option<Regex>)>, model: &str, firmware: &str) -> Option<&'a Entry> {
	for &(entry, ref model_re, ref fw_re) in db.iter() {
		if ! model_re.is_match(model.as_bytes()) { continue }

		if let &Some(ref fw_re) = fw_re {
			if ! fw_re.is_match(firmware.as_bytes()) { continue }
		}

		return Some(entry);
	}

	None
}

fn find_precompiled_s<'a>(db: &Vec<(&'a Entry, LazyRegex, Option<LazyRegex>)>, model: &str, firmware: &str) -> Option<&'a Entry> {
	for &(entry, ref model_re, ref fw_re) in db.iter() {
		if ! model_re.is_match(model) { continue }

		if let &Some(ref fw_re) = fw_re {
			if ! fw_re.is_match(firmware) { continue }
		}

		return Some(entry);
	}

	None
}

fn find_precompiled_set(models: &RegexSet, firmwares: &RegexSet, model: &str, firmware: &str) -> Option<usize> {
	let models: HashSet<_> = models.matches(model.as_bytes()).iter().collect();
	let firmwares: HashSet<_> = firmwares.matches(firmware.as_bytes()).iter().collect();

	models.intersection(&firmwares).min().map(|index| *index)
}

fn main() {
	let args = App::new("drivedb-bench")
		.version(crate_version!())
		.arg(Arg::with_name("drivedb")
			.required(true)
			.help("paths to drivedb file"))
		.arg(Arg::with_name("model")
			.required(true)
		)
		.arg(Arg::with_name("firmware")
			.required(true)
		)
		.get_matches();

	let drivedb = args.value_of("drivedb").unwrap();
	let model = args.value_of("model").unwrap();
	let firmware = args.value_of("firmware").unwrap();

	let now = Instant::now();
	let drivedb = drivedb::load(drivedb);
	println!("loaded drivedb in {:.4}ms", elapsed(now));
	let drivedb = drivedb.unwrap();

	let now = Instant::now();
	let e = find(&drivedb, &model, &firmware);
	println!("find() in {:.4}ms", elapsed(now));
	println!("{:?}", e);

	for _ in 1..10 {
		let now = Instant::now();
		let e = find(&drivedb, &model, &firmware);
		println!("find() in {:.4}ms", elapsed(now));
		black_box(e); // make sure `e` is not eliminated
	}

	let now = Instant::now();
	let compiled: Vec<_> = drivedb.iter()
		.filter(|e| ! e.model.starts_with("USB:"))
		.map(|e| (
			e,
			Regex::new(format!("(?-u)^{}$", e.model).as_str()).unwrap(),
			if e.firmware.is_empty() { None } else {
				Some(Regex::new(format!("^(?-u){}$", e.firmware).as_str()).unwrap())
			},
		))
		.collect();
	println!("compiled in {:.4}ms", elapsed(now));

	let now = Instant::now();
	let e = find_precompiled(&compiled, &model, &firmware);
	println!("find_precompiled() in {:.4}ms", elapsed(now));
	println!("{:?}", e);

	for _ in 1..10 {
		let now = Instant::now();
		let e = find_precompiled(&compiled, &model, &firmware);
		println!("find_precompiled() in {:.4}ms", elapsed(now));
		black_box(e); // make sure `e` is not eliminated
	}

	let now = Instant::now();
	let compiled: Vec<_> = drivedb.iter()
		.filter(|e| ! e.model.starts_with("USB:"))
		.map(|e| (
			e,
			LazyRegex::new(format!("^{}$", e.model).as_str()).unwrap(),
			if e.firmware.is_empty() { None } else {
				Some(LazyRegex::new(format!("^{}$", e.firmware).as_str()).unwrap())
			},
		))
		.collect();
	println!("compiled lazy in {:.4}ms", elapsed(now));

	let now = Instant::now();
	let e = find_precompiled_s(&compiled, &model, &firmware);
	println!("find_precompiled_s() in {:.4}ms", elapsed(now));
	println!("{:?}", e);

	for _ in 1..10 {
		let now = Instant::now();
		let e = find_precompiled_s(&compiled, &model, &firmware);
		println!("find_precompiled_s() in {:.4}ms", elapsed(now));
		black_box(e); // make sure `e` is not eliminated
	}

	let now = Instant::now();
	let models = RegexSet::new(drivedb.iter()
		.filter(|e| ! e.model.starts_with("USB:"))
		.map(|e| format!("^{}$", e.model))
	).unwrap();
	let firmwares = RegexSet::new(drivedb.iter()
		.filter(|e| ! e.model.starts_with("USB:"))
		.map(|e| if e.firmware.is_empty() { "".to_string() } else {
			format!("^{}$", e.firmware)
		})
	).unwrap();
	println!("compiled set in {:.4}ms", elapsed(now));

	let now = Instant::now();
	let e = find_precompiled_set(&models, &firmwares, &model, &firmware);
	println!("find_precompiled_set() in {:.4}ms", elapsed(now));
	println!("{:?}", e.map(|index| &drivedb[index]));

	for _ in 1..10 {
		let now = Instant::now();
		let e = find_precompiled_set(&models, &firmwares, &model, &firmware);
		println!("find_precompiled_set() in {:.4}ms", elapsed(now));
		black_box(e); // make sure `e` is not eliminated
	}
}
