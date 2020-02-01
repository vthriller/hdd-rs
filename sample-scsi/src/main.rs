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
use hdd::scsi::{SCSIDevice, SCSICommon};
use hdd::scsi::pages::{SCSIPages, page_name};
use hdd::scsi::data::inquiry;
use hdd::scsi::data::vpd::device_id;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate separator;
use separator::Separatable;
extern crate number_prefix;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

#[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
fn print_hex(data: &[u8]) {
	for i in 0..data.len() {
		if i % 16 == 0 { print!("\n"); }
		print!(" {:02x}", data[i]);
	}
	print!("\n");
}

fn query(what: &str, dev: &SCSIDevice, vpd: bool, page: u8, verbose: bool) -> Vec<u8> {
	print!("=== {} ===\n", what);
	let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data: len={}", data.len());
		print_hex(&data);
	}

	data
}

fn main() {
	let args = App::new("sample-scsi")
		.version(crate_version!())
		.arg(Arg::with_name("device")
			.help("Device to query")
			.required(true)
			.index(1)
		)
		.arg(Arg::with_name("verbose")
			.short("v")
			.long("verbose")
			.help("show hex data")
		)
		.get_matches();

	let dev = SCSIDevice::new(Device::open(
		args.value_of("device").unwrap()
	).unwrap());
	let verbose = args.is_present("verbose");

	let (_, lba, block_size) = dev.read_capacity_10(None).unwrap();
	let cap = lba as u64 * block_size as u64;
	print!("Capacity: {} × {}\n", lba, block_size);
	print!("          {} bytes\n", cap.separated_string());
	print!("          ({}, {})\n",
		match decimal_prefix(cap as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
		match binary_prefix(cap as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
	);

	let data = query("Inquiry", &dev, false, 0, verbose);
	print!("{:#?}\n", inquiry::parse_inquiry(&data));

	let data = query("[00] Supported VPD pages", &dev, true, 0, verbose);
	let len = data[3];
	print!("supported:");
	for i in 0..len {
		print!(" {:02x}", data[(i+4) as usize]);
	}
	print!("\n");

	let data = query("[83] Device Information", &dev, true, 0x83, verbose);
	let len = ((data[2] as usize) << 8) + (data[3] as usize);

	print!("descriptors:\n");
	for d in device_id::parse(&data[4 .. 4+len]) {
		print!("{:?}\n", d);

		// TODO? from_utf8 it right in hdd::data::vpd::device_id
		if d.codeset == device_id::CodeSet::ASCII {
			match d.id {
				device_id::Identifier::VendorSpecific(i) |
				device_id::Identifier::FCNameIdentifier(i) => {
					print!(">>> {:?}\n", std::str::from_utf8(i));
				},
				device_id::Identifier::Generic { vendor_id: v, id: i } => {
					print!(">>> {:?}\n", std::str::from_utf8(v));
					print!(">>> {:?}\n", std::str::from_utf8(i));
				},
				_ => (),
			}
		}
	}

	match SCSIPages::new(&dev) {
		Err(e) => eprint!("failed to query supported pages: {}", e),
		Ok(mut pages) => {
			for p in pages.supported_pages().to_owned() {
				if p == 00 { continue; }

				print!("=== [{:02x}] {} ===\n", p, page_name(p));
				match p {
					// already in cli
					0x02...0x06 | 0x0d | 0x0e => (),
					0x10 => print!("{:#?}\n", pages.self_test_results()),
					0x2f => print!("{:#?}\n", pages.informational_exceptions()),
					_ => (),
				}
			}
		}
	}

	/*
	// TODO tell whether subpages are supported at all
	let data = ask_log("[00/ff] Supported Log Pages/Subpages", &dev, 0x00, 0xff, verbose);
	let page = log_page::parse(&data);
	if let Some(page) = page {
		for psp in page.data[..].chunks(2) {
			let (page, subpage) = (psp[0], psp[1]);

			let data = ask_log(&format!("[{:02x}/{:02x}] ?", page, subpage), &dev, page, subpage, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				print!("{:?}\n", page);
				print!("{:#?}\n", page.parse_params());
			}
		}
	}
	*/
}
