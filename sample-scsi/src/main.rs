extern crate hdd;
use hdd::Device;
use hdd::scsi::SCSIDevice;
use hdd::scsi::data::{inquiry, log_page};
use hdd::scsi::data::vpd::device_id;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate separator;
use separator::Separatable;
extern crate number_prefix;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

extern crate byteorder;
use byteorder::{ReadBytesExt, BigEndian};

fn print_hex(data: &[u8]) {
	for i in 0..data.len() {
		if i % 16 == 0 { print!("\n"); }
		print!(" {:02x}", data[i]);
	}
	print!("\n");
}

fn query(what: &str, dev: &Device, vpd: bool, page: u8, verbose: bool) -> Vec<u8> {
	print!("=== {} ===\n", what);
	let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data:");
		print_hex(&data);
	}

	data
}

fn ask_log(what: &str, dev: &Device, page: u8, subpage: u8, verbose: bool) -> Vec<u8> {
	print!("=== {} ===\n", what);
	let (sense, data) = dev.log_sense(
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

	let dev = Device::open(
		args.value_of("device").unwrap()
	).unwrap();
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

	let data = ask_log("[00] Supported Log Pages", &dev, 0x00, 0x00, verbose);
	let page = log_page::parse(&data);
	if let Some(page) = page {
		for p in page.data {
			if *p == 00 { continue; }

			let name = match *p {
				0x02 => "Write Error Counter",
				0x03 => "Read Error Counter",
				0x04 => "Read Reverse Error Counter",
				0x05 => "Verify Error Counter",
				0x06 => "Non-Medium Error",
				0x0d => "Temperature",
				0x0e => "Start-Stop Cycle Counter",
				0x10 => "Self-Test results",
				0x2f => "Informational Exceptions",
				0x30...0x3e => "(Vendor-Specific)",
				0x3f => "(Reserved)",
				_ => "?",
			};

			let err_cnt_desc = |x| match x {
				0000 => "Errors corrected without substantial delay".to_string(),
				0001 => "Errors corrected with possible delays".to_string(),
				0002 => "Total (e.g., rewrites or rereads)".to_string(),
				0003 => "Total errors corrected".to_string(),
				0004 => "Total times correction algorithm processed".to_string(),
				0005 => "Total bytes processed".to_string(),
				0006 => "Total uncorrected errors".to_string(),
				x @ 0x8000...0xffff => format!("(Vendor specific) {}", x),
				x => format!("(Reserved) {}", x),
			};

			let data = ask_log(&format!("[{:02x}] {}", p, name), &dev, *p, 0x00, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				match *p {
					0x02...0x05 => { // xxx Error Counter
						if let Some(params) = page.parse_params() {
							for param in params {
								// XXX tell about unexpected params?
								if param.value.len() == 0 { continue; }

								print!("{}: {}\n",
									err_cnt_desc(param.code),
									(&param.value[..]).read_uint::<BigEndian>(param.value.len()).unwrap(),
								);
							}
						}
					},
					0x06 => { // Non-Medium Error Count
						if let Some(params) = page.parse_params() {
							for param in params {
								// XXX tell about unexpected params?
								if param.value.len() == 0 { continue; }
								if param.code != 0 { continue; }

								print!("Error Count: {}\n",
									(&param.value[..]).read_uint::<BigEndian>(param.value.len()).unwrap(),
								);
							}
						}
					},
					0x0d => { // Temperature
						if let Some(params) = page.parse_params() {
							for param in params {
								// XXX tell about unexpected params?
								if param.value.len() < 2 { continue; }

								// value[0] is reserved
								print!("{}: {} C\n", match param.code {
									0x0000 => "Temperature",
									0x0001 => "Reference Temperature", // maximum temperature at which device is capable of operating continuously without degrading
									_ => "?",
								}, match param.value[1] {
									0xff => continue, // unable to return temperature despite including this param in the answer
									x => x,
								});
							}
						}
					},
					0x0e => { // Start-Stop Cycle Counter
						if let Some(params) = page.parse_params() {
							for param in params {
								match param.code {
									0x0001 => {
										// XXX tell about unexpected params?
										if param.value.len() < 6 { continue; }
										print!("Date of manufacturing: week {} of {}\n",
											String::from_utf8(param.value[4..6].to_vec()).unwrap(), // ASCII
											String::from_utf8(param.value[0..4].to_vec()).unwrap(), // ASCII
										);
									},
									0x0002 => {
										// XXX tell about unexpected params?
										if param.value.len() < 6 { continue; }
										print!("Accounting Date: week {} of {}\n", // in which the device was placed in service
											String::from_utf8(param.value[4..6].to_vec()).unwrap(), // ASCII, might be all-spaces
											String::from_utf8(param.value[0..4].to_vec()).unwrap(), // ASCII, might be all-spaces
										);
									},
									0x0003 => {
										if param.value.len() < 4 { continue; }
										print!("Specified Cycle Count Over Device Lifetime: {}\n",
											(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
										);
									},
									0x0004 => {
										if param.value.len() < 4 { continue; }
										print!("Accumulated Start-Stop Cycles: {}\n",
											(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
										);
									},
									0x0005 => {
										if param.value.len() < 4 { continue; }
										print!("Specified Load-Unload Count Over Device Lifetime: {}\n",
											(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
										);
									},
									0x0006 => {
										if param.value.len() < 4 { continue; }
										print!("Accumulated Load-Unload Cycles: {}\n",
											(&param.value[0 .. 4]).read_u32::<BigEndian>().unwrap()
										);
									},
									_ => {
										print!("? {:?}\n", param);
									},
								}
							}
						}
					},
					0x10 => { // Self-Test results
						if let Some(params) = page.parse_params() {
							for param in params {
								// XXX tell about unexpected params?
								if param.code == 0 || param.code > 0x0014 { continue; }
								if param.value.len() < 0x10 { continue; }

								// unused self-test log parameter is all zeroes
								if *param.value.iter().max().unwrap() == 0 { continue }

								print!("self-test:\n");
								print!("result = {}\n", match param.value[0] & 0b111 {
									0 => "no error".to_string(),
									1 => "aborted explicitly".to_string(),
									2 => "aborted by other means".to_string(),
									3 => "unknown error occurred".to_string(),
									4 => "failed (unknown segment)".to_string(),
									5 => "failed (1st segment)".to_string(),
									6 => "failed (2nd segment)".to_string(),
									7 => "failed (other segment)".to_string(),
									15 => "in progress".to_string(),
									x => format!("(reserved) {}", x),
								});
								print!("test code: {}\n", (param.value[0] & 0b11100000) >> 5);
								print!("test number: {}\n", param.value[1]);
								let hours = (&param.value[2..4]).read_u16::<BigEndian>().unwrap();
								if hours == 0xffff {
									print!("accumulated power-on hours: > {}\n", hours);
								} else {
									print!("accumulated power-on hours: {}\n", hours);
								}
								print!("address of first failure: {:016x}\n",
									(&param.value[4..12]).read_u64::<BigEndian>().unwrap(),
								);
								print!("sense key/ASC/ASCQ: {} {} {}\n",
									param.value[12] & 0b1111,
									param.value[13],
									param.value[14],
								);
								print!("(vendor specific): {}\n", param.value[15]);
							}
						}
					},
					0x2f => { // Informational Exceptions
						if let Some(params) = page.parse_params() {
							for param in params {
								// XXX tell about unexpected params?
								if param.code != 0 { continue; }
								if param.value.len() < 3 { continue; }

								print!("IE ASC: {:02x}\n", param.value[0]);
								print!("IE ASCQ: {:02x}\n", param.value[1]);
								print!("Most Recent Temperature Reading: {}\n", param.value[2]);
								print!("(Vendor-specific): len={}\n", param.value[3..].len());
								print_hex(&param.value[3..]);
							}
						}
					},
					_ => {
						print!("{:?}\n", page);
						print!("{:#?}\n", page.parse_params());
					},
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
