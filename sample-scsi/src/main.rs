extern crate hdd;
use hdd::scsi;
use hdd::data::inquiry;
use hdd::data::vpd::device_id;
use hdd::data::log_page;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate separator;
use separator::Separatable;
extern crate number_prefix;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

fn print_hex(data: &[u8]) {
	for i in 0..data.len() {
		if i % 16 == 0 { print!("\n"); }
		print!(" {:02x}", data[i]);
	}
	print!("\n");
}

fn query(what: &str, file: &str, vpd: bool, page: u8, verbose: bool) -> [u8; 4096] {
	print!("=== {} ===\n", what);
	let (sense, data) = scsi::scsi_inquiry(&file, vpd, page).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data:");
		print_hex(&data);
	}

	data
}

fn ask_log(what: &str, file: &str, page: u8, subpage: u8, verbose: bool) -> [u8; 4096] {
	print!("=== {} ===\n", what);
	let (sense, data) = scsi::log_sense(&file,
		false, // changed
		false, // save_params
		false, // default
		false, // threshold
		page, subpage,
		0, // param_ptr
	).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data:");
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

	let file = args.value_of("device").unwrap();
	let verbose = args.is_present("verbose");

	let (_, lba, block_size) = scsi::read_capacity_10(file, None).unwrap();
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

	let data = query("Inquiry", &file, false, 0, verbose);
	print!("{:#?}\n", inquiry::parse_inquiry(&data));

	let data = query("[00] Supported VPD pages", &file, true, 0, verbose);
	let len = data[3];
	print!("supported:");
	for i in 0..len {
		print!(" {:02x}", data[(i+4) as usize]);
	}
	print!("\n");

	let data = query("[83] Device Information", &file, true, 0x83, verbose);
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

	let data = ask_log("[00] Supported Log Pages", &file, 0x00, 0x00, verbose);
	let page = log_page::parse(&data);
	if let Some(page) = page {
		for p in page.data {
			let data = ask_log(&format!("[{:02x}] ?", p), &file, *p, 0x00, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				print!("{:?}\n", page);
				print!("{:#?}\n", page.parse_params());
			}
		}
	}

	/*
	// TODO tell whether subpages are supported at all
	let data = ask_log("[00/ff] Supported Log Pages/Subpages", &file, 0x00, 0xff, verbose);
	let page = log_page::parse(&data);
	if let Some(page) = page {
		for psp in page.data[..].chunks(2) {
			let (page, subpage) = (psp[0], psp[1]);

			let data = ask_log(&format!("[{:02x}/{:02x}] ?", page, subpage), &file, page, subpage, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				print!("{:?}\n", page);
				print!("{:#?}\n", page.parse_params());
			}
		}
	}
	*/
}
