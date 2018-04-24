use cam::{CAMDevice, CCB, error};
use cam::bindings::{xpt_opcode, cam_status, cam_proto};
use std::io;
use std::path::PathBuf;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub(crate) dev: CAMDevice,
}

#[derive(Debug)]
pub enum Type { ATA, SCSI }

impl Device {
	pub fn open(path: &str) -> Result<Self, io::Error> {
		Ok(Device {
			dev: CAMDevice::open(path)?,
		})
	}

	pub fn get_type(&self) -> Result<Type, io::Error> {
		unsafe {
			let ccb: CCB = CCB::new(&self.dev);
			ccb.ccb_h().func_code = xpt_opcode::XPT_PATH_INQ;

			self.dev.send_ccb(&ccb)?;

			if ccb.get_status() != cam_status::CAM_REQ_CMP as u32 {
				Err(error::from_status(&self.dev, &ccb))?
			}

			use self::cam_proto::*;

			Ok(match ccb.cpi().protocol {
				// TODO USB, SATA port multipliers and whatnot
				PROTO_ATA => Type::ATA,
				_ => Type::SCSI,
			})
		}
	}
}

// based on freebsd/sbin/camcontrol/camcontrol.c:getdevtree (`camcontrol devlist`), smartmontools/os_freebsd.cpp:get_dev_names_cam
// TODO? smartmontools also try to probe /dev/ata; should we do the same, or is it just some deprecated thing?
pub fn list_devices() -> Result<Vec<PathBuf>, io::Error> {
	use cam::bindings::{
		ccb,
		CAMIOCOMMAND, XPT_DEVICE,
		CAM_XPT_PATH_ID, CAM_TARGET_WILDCARD, CAM_LUN_WILDCARD,
		cam_status_CAM_STATUS_MASK, cam_status,
		ccb_dev_match_status, dev_match_result,
	};
	use libc::ioctl;
	use std::mem;
	use std::ffi::CStr;
	use std::fs::OpenOptions;
	use std::os::unix::io::AsRawFd;

	/*
	If, looking at the CAMIOCOMMAND ioctl call, you thought:

	— Ahaa, that's exactly what `send_ccb()` does!

	and decided to simplify every other line of this function with wrappers from `mod cam`, then I'm about to disappoint you.

	- `cam_open_device()` does a lot of things under the hood, like resolving real device path, passthrough device, bus number etc.
	  All these things fail for xpt(4) device for some reason, thus we don't use `CAMDevice::open(&xpt)`.
	- `cam_getccb()` requires valid CAM device, with path_id/target_id/target_lun and whatnot, so we cannot use `struct CCB` wrapper either.
	- The same applies to `cam::error`: native error string formatter passes CAM device to `cam_path_string()`, which in turn relies on heck of a lot of values from that device.
	 (Again, all we have is a file descriptor, not a full struct.)
	- Can we fake device, allocating it manually and filling with the fields we want? Sure! But:
	  - `impl Drop` will eventually pass our struct to `free()` from libc, and things allocated with Rust's allocator must also be freed using the same allocator,
	  - thus we'll probably need another (set?) of helper structs for Rust-allocated structs just for this very function, and that would be an overkill,
	  - also, faking device_name/sim_unit_number/etc. would only result in confusing errors (remember what `cam_path_string()` renders!).

	If for whatever reason you missed the mess that Linux's sysfs sometimes is, well, make yourself at home!
	*/

	info!("listing devices through xpt(4)");

	// XPT_DEVICE is an… &[u8]. With trailing \0. …yeah.
	let xpt = String::from_utf8(
		XPT_DEVICE[..XPT_DEVICE.len()-1].to_vec()
	).unwrap();
	let xpt = OpenOptions::new()
		.read(true)
		.write(true)
		.open(xpt)?;

	// smartmontools defaults to 26, camcontrol hard-codes the number 100 here
	const MAX_NUM_DEV: usize = 64;
	let mut matches = Vec::with_capacity(MAX_NUM_DEV);

	let mut ccb = unsafe {
		let mut ccb = mem::zeroed::<ccb>();

		ccb.ccb_h.as_mut().path_id = CAM_XPT_PATH_ID;
		ccb.ccb_h.as_mut().target_id = CAM_TARGET_WILDCARD;
		// XXX without .into(): 'expected u64, found u32'
		// CAM_LUN_WILDCARD is defined as u_int, target_lun though is lun_id_t, which in turn is u_int64_t. Oh boy :\
		ccb.ccb_h.as_mut().target_lun = CAM_LUN_WILDCARD.into();

		ccb.ccb_h.as_mut().func_code = xpt_opcode::XPT_DEV_MATCH;

		ccb.cdm.as_mut().match_buf_len = (matches.capacity() * mem::size_of::<dev_match_result>()) as u32;
		ccb.cdm.as_mut().matches = matches.as_mut_ptr();
		// smartmontools also bzero()es `matches` here; camcontrol, however, doesn't

		ccb.cdm.as_mut().num_matches = 0;
		ccb.cdm.as_mut().num_patterns = 0;
		ccb.cdm.as_mut().pattern_buf_len = 0;

		ccb
	};

	/*
	In smartmontools array of matches is interpreted as some sort of a flattened tree, e.g.

		bus1 dev11 periph111 dev12 periph121 periph122 bus2 dev21 periph211 periph212

	is interpreted as if it was

		bus1 ( dev11 ( periph111 ) dev12 ( periph121 periph122 ) ) bus2 ( dev21 ( periph211 periph212 ) )

	In other words, periphery devices are picked based on what the latest bus/device was.

	`camcontrol devlist [-v]` behaves similarly: it just outputs information on each item regardless of its type (bus/dev/periph), without any sorting.

	Multiple periphery devices are like alternative device names for the top device (which apparently has no name);
	smartmontools prioritizes the one that is not 'passX'
	(linux analogy: preferring '/sys/class/block/sda' over '/sys/class/scsi_generic/sg0',
	 although both 'sda' and 'sg0' can be used to access the device in question)
	*/

	/*
	vector of devices, or, rather, all the names for each device found
	e.g.
	vec![
		vec![("cd", 0), ("pass", 0)],
		vec![("pass", 1), ("da", 0)],
		vec![], // skipped device or whatever
	]
	*/
	let mut devices: Vec<Vec<(String, u32)>> = vec![];

	// > We do the ioctl multiple times if necessary, in case there are more than MAX_NUM_DEV nodes in the EDT.
	loop {
		debug!("(CAMIOCOMMAND)");
		if unsafe { ioctl(xpt.as_raw_fd(), CAMIOCOMMAND, &mut ccb) == -1 } {
			return Err(io::Error::last_os_error());
		}

		let cam_status = unsafe { ccb.ccb_h.as_ref().status };
		let cdm_status = unsafe { ccb.cdm.as_ref().status };
		let err = || Err(io::Error::new(io::ErrorKind::Other,
			format!("CAM error 0x{:x}, CDM error {}\n",
				cam_status,
				cdm_status as u32,
			)
		));

		// see also CCB.get_status()
		if (cam_status & cam_status_CAM_STATUS_MASK as u32) != (cam_status::CAM_REQ_CMP as u32) {
			return err();
		}
		match cdm_status {
			ccb_dev_match_status::CAM_DEV_MATCH_LAST => (),
			ccb_dev_match_status::CAM_DEV_MATCH_MORE => (),
			_ => return err(),
		}

		unsafe { matches.set_len(ccb.cdm.as_ref().num_matches as usize) }

		let mut skip_bus = false;
		let mut skip_dev = false;
		for m in matches.iter() { // don't consume it, we'll fill it with another page
			use cam::bindings::dev_match_type::*;
			use cam::bindings::dev_result_flags::DEV_RESULT_UNCONFIGURED;
			match m.type_ {
				/*
				`CStr::from_ptr(whatever.as_ref().as_ptr())` might seem weird and a bit redundant,
				but `CStr::from_bytes_with_nul(whatever.as_ref())` cannot be used due to the fact that
				`CStr` only works with `[u8]`, but
				`char whatever[…]` is rendered as `whatever: [c_char; …]` by bindgen (obviously),
				which in turn might end up being either `[i8; …]` or `[u8; …]` depending on the platform

				`unsafe { CStr::from_ptr() }` is safe: `matches` outlives any of the pointers passed in.
				HOWEVER, any string that should survive consecutive CAMIOCOMMANDs must be cloned because of reused `matches`.
				*/
				DEV_MATCH_BUS => {
					let bus = unsafe { m.result.bus_result.as_ref() };
					let bus_name = unsafe { CStr::from_ptr(bus.dev_name.as_ref().as_ptr()) };
					debug!("bus {:?}", bus_name);

					skip_bus = match bus_name.to_str() {
						Ok("xpt") => true,
						Err(e) => { // should never happen
							debug!("  failed to parse bus name: {}", e);
							true
						},
						_ => false,
					};
					if skip_bus { debug!("  skip"); }
				},
				DEV_MATCH_DEVICE => {
					let dev = unsafe { m.result.device_result.as_ref() };
					debug!("  dev flags=0x{:x}", dev.flags as usize);

					devices.push(vec![]);

					// TODO? skip devices based on dev.protocol value
					skip_dev = skip_bus || (dev.flags as usize & DEV_RESULT_UNCONFIGURED as usize != 0);
					if skip_dev { debug!("    skip"); }
				},
				DEV_MATCH_PERIPH => {
					let pdev = unsafe { m.result.periph_result.as_ref() };
					let pname = unsafe { CStr::from_ptr(pdev.periph_name.as_ref().as_ptr()) };
					debug!("    periph {:?} {}", pname, pdev.unit_number);

					if ! skip_dev {
						match pname.to_str() {
							Ok(pname) => match devices.last_mut() { // latest device added in DEV_MATCH_DEVICE match arm
								Some(last_dev) =>
									last_dev.push((pname.to_string(), pdev.unit_number)),
								None => {
									// IMPOSSIBRU?
									return Err(io::Error::new(io::ErrorKind::Other, "list_devices: peripheral device appeared before any actual device"))
								},
							},
							// should never happen:
							Err(e) => debug!("      failed to parse pname: {}", e),
						}
					} else {
						debug!("      skipped");
					}
				},
			}
		}

		if cdm_status == ccb_dev_match_status::CAM_DEV_MATCH_LAST { break }
	}

	// and now, cherry-picking device names
	let devices = devices.into_iter()
		.filter_map(|names| {
			let mut names = names.into_iter();
			// pick the first name as the initial
			// if there's no names, next() will return None, and filter_map() will just skip this device
			names.next().map(|init|
				// for any other name, if there's any name that is not 'passX', pick that name over the current one
				names.fold(init, |prev, current|
					if current.0 != "pass" { current }
					else { prev }
				)
			)
		})
		.map(|(name, unit)| PathBuf::from(format!("/dev/{}{}", name, unit)))
		.collect();

	Ok(devices)
}
