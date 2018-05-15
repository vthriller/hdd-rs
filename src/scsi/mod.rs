/*!
All things SCSI.

* Use [`struct SCSIDevice`](struct.SCSIDevice.html) + [`trait SCSICommon`](trait.SCSICommon.html) to start sending SCSI commands to the [`Device`](../device/index.html).
* Use [`data` module](data/index.html) to parse various low-level structures found in SCSI command replies.
* Import traits from porcelain modules (like [`pages`](pages/index.html)) to do typical tasks without needing to compose commands and parse responses yourself.
  * You can also use [`module ata`](../ata/index.html) to issue ATA commands using ATA PASS-THROUGH.
*/

pub mod data;
pub mod pages;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

use std::io;
use ata;
use byteorder::{ReadBytesExt, BigEndian};
use self::data::sense;

use Direction;
use Device;

use utils::hexdump_8;

quick_error! {
	#[derive(Debug)]
	pub enum Error {
		IO(err: io::Error) {
			from()
			display("IO error: {}", err)
			description(err.description())
			cause(err)
		}
		// XXX make sure only non-deferred senses are used here
		// XXX it makes no sense (sorry!) to put informational senses here (i.e. sense::SenseKey::{Ok, Recovered, Completed})
		Sense(key: sense::key::SenseKey, asc: u8, ascq: u8) { // XXX do we need additional sense data? descriptors? flags? probably not
			// FIXME no from() here due to sense::Sense lifetimes; for now use Error::from_sense() instead
			description("SCSI error")
			display("SCSI error: {:?} ({})",
				key,
				sense::key::decode_asc(*asc, *ascq)
					.map(|x| x.to_string())
					.unwrap_or_else(|| format!("unknown additional sense code: {:02x} {:02x}", asc, ascq)))
		}
		// this is for Sense::Fixed(FixedData::Invalid(_))
		// pun definitely intented at this point
		Nonsense {}
	}
}

impl Error {
	fn from_sense(sense: &sense::Sense) -> Self {
		match sense.kcq() {
			Some((key, asc, ascq)) =>
				Error::Sense(sense::key::SenseKey::from(key), asc, ascq),
			None =>
				Error::Nonsense,
		}
	}
}

// FIXME naming: this isn't about ATA-level error, this is error related to ATA PASS-THROUGH command
quick_error! {
	#[derive(Debug)]
	pub enum ATAError {
		SCSI(err: Error) {
			from()
			from(err: io::Error) -> (Error::IO(err))
			display("{}", err)
		}
		/// Device does not support ATA PASS-THROUGH command
		NotSupported {}
		// no non-deferred sense is available, or there's no descriptors for ATA registers to be found
		NoRegisters {}
	}
}

#[derive(Debug)]
pub struct SCSIDevice {
	device: Device,
}

impl SCSIDevice {
	pub fn new(device: Device) -> Self {
		Self { device }
	}

	// thin wrapper against platform-specific implementation, mainly exists to provide consistent logging between platforms
	/// Executes `cmd` and returns tuple of `(sense, data)`.
	pub fn do_cmd(&self, cmd: &[u8], dir: Direction, sense_len: usize, data_len: usize) -> Result<(Vec<u8>, Vec<u8>), io::Error> {
		info!("SCSI cmd: dir={:?} cmd={:?}", dir, cmd);

		// this one is implemented in `mod {linux,freebsd}`
		let ret = Self::do_platform_cmd(self, cmd, dir, sense_len, data_len);
		match ret {
			Ok((ref sense, ref data)) => {
				debug!("SCSI autosense: {}", hexdump_8(sense));
				debug!("SCSI data: {}", hexdump_8(data));
			},
			ref err => {
				debug!("SCSI err: {:?}", err);
			}
		}
		ret
	}
}

// TODO pub? see read_defect_data_*()
#[derive(Debug)]
enum AddrDescriptorFormat {
	ShortBlock = 0b000,
	LongBlock = 0b011,
	BytesFromIndex = 0b100,
	PhysSector = 0b101,
	VendorSpecific = 0b110,
	// others are reserved
}

#[derive(Debug, PartialEq)]
pub enum DefectList {
	Primary,
	Grown,
	Both,
	// why would anyone send READ DEFECT DATA with req_{p,g}list set to 0?
}

// TODO look for non-empty autosense and turn it into errors where appropriate
pub trait SCSICommon {
	// XXX DRY
	fn do_cmd(&self, cmd: &[u8], dir: Direction, sense_len: usize, data_len: usize) -> Result<(Vec<u8>, Vec<u8>), io::Error>;

	fn scsi_inquiry(&self, vital: bool, code: u8) -> Result<(Vec<u8>, Vec<u8>), Error> {
		info!("issuing INQUIRY: code={:?} vital={:?}", code, vital);

		// TODO as u16 argument, not const
		const alloc: usize = 4096;

		let cmd: [u8; 6] = [
			0x12, // opcode: INQUIRY
			if vital {1} else {0}, // reserved << 2 + cmddt (obsolete) << 1 + enable vital product data << 0
			code,
			(alloc >> 8) as u8,
			(alloc & 0xff) as u8,
			0, // control (XXX what's that?!)
		];

		Ok(self.do_cmd(&cmd, Direction::From, 32, alloc)?)
	}

	/// returns tuple of (sense, logical block address, block length in bytes)
	fn read_capacity_10(&self, lba: Option<u32>) -> Result<(Vec<u8>, u32, u32), Error> {
		info!("issuing READ CAPACITY(10): lba={:?}", lba);

		// pmi is partial medium indicator
		let (pmi, lba) = match lba {
			Some(lba) => (true, lba),
			None => (false, 0),
		};

		let cmd: [u8; 10] = [
			0x25, // opcode
			0, // reserved, obsolete
			((lba >> 24) & 0xff) as u8,
			((lba >> 16) & 0xff) as u8,
			((lba >> 8)  & 0xff) as u8,
			((lba)       & 0xff) as u8,
			0, // reserved
			0, // reserved
			if pmi { 1 } else { 0 }, // reserved, pmi
			0, // control (XXX what's that?!)
		];

		let (sense, data) = self.do_cmd(&cmd, Direction::From, 32, 8)?;

		Ok((
			sense,
			(&data[0..4]).read_u32::<BigEndian>().unwrap(),
			(&data[4..8]).read_u32::<BigEndian>().unwrap(),
		))
	}

	// not returning actual defects list because it seems to be useless for the average user
	// hence no `format` arg
	/**
	Executes READ DEFECT DATA(10) command, returning the number of entries in the `list`.

	Returns `None` if:

	- device returns entries it wasn't asked to list (e.g. primary list for `DefectList::Grown`)
	- device returns entries in unexpected format
	- device returns malformed data
	*/
	fn read_defect_data_10(&self, list: DefectList) -> Result<Option<u16>, Error> {
		// XXX tried (Short|Long)Block on HUS723030ALS640, got GROWN DEFECT LIST NOT FOUND in return—why?
		// for now use the same format smartmontools uses from time immemorial
		let format = AddrDescriptorFormat::BytesFromIndex;

		let (plist, glist) = match list {
			DefectList::Primary => (true,  false),
			DefectList::Grown   => (false, true),
			DefectList::Both    => (true,  true),
		};

		info!("issuing READ DEFECT DATA(10): plist={:?} glist={:?} format={:?}", plist, glist, format);

		let plist = if plist { 1 } else { 0 };
		let glist = if glist { 1 } else { 0 };

		// we're only interested in the header, not the list itself
		const alloc: usize = 4;

		let cmd: [u8; 10] = [
			0x37, // opcode
			0, // reserved
			(plist << 4) + (glist << 3) + (format as u8), // reserved (3 bits), req_plist, req_glist, defect list format (3 bits)
			0, 0, 0, 0, // reserved
			(alloc >> 8) as u8,
			(alloc & 0xff) as u8,
			0, // control (XXX what's that?!)
		];

		let (sense, data) = self.do_cmd(&cmd, Direction::From, 32, alloc)?;

		if sense.len() > 0 {
			// only current senses are expected here
			if let Some((true, sense)) = sense::parse(&sense) {
				match sense.kcq() {
					// DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x00)) => return Ok(Some(0)),
					// PRIMARY DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x01)) |
					// GROWN DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x02)) => {
						if list != DefectList::Both {
							return Ok(Some(0))
						} // else fall through to the data parser
						// XXX is it correct to just dismiss (WHATEVER) DEFECT LIST NOT FOUND if DefectList::Both is requested?
					},
					// unexpected sense
					s => return Err(Error::from_sense(&sense)),
				}
			}
		}

		if let Some((format, glistv, plistv, len)) = parse_defect_data_10(&data) {
			debug!("defect list: format={} glistv={} plistv={} len={}\n", format, glistv, plistv, len);

			match (list, plistv, glistv) {
				(DefectList::Primary, true,  false) => (),
				(DefectList::Grown,   false, true)  => (),
				(DefectList::Both,    _,     _)     => (), // XXX match true/true?
				_ => {
					info!("device returned unexpected defect list");
					return Ok(None);
				},
			}

			// see SBC-3, 5.2.2.4
			let entry_size = match format {
				0b000 => 4, // ShortBlock
				0b011 => 8, // LongBlock
				0b100 => 8, // BytesFromIndex
				0b101 => 8, // PhysSector
				_ => {
					info!("unexpected defect list entry format");
					return Ok(None);
				},
			};

			return Ok(Some(len / entry_size));
		} else {
			info!("defect list: not enough data");
			return Ok(None);
		}
	}

	/**
	Executes READ DEFECT DATA(12) command, returning the number of entries in the `list`.

	Returns `None` if:

	- device returns entries it wasn't asked to list (e.g. primary list for `DefectList::Grown`)
	- device returns entries in unexpected format
	- device returns malformed data
	*/
	fn read_defect_data_12(&self, list: DefectList) -> Result<Option<u32>, Error> {
		// XXX see read_defect_data_10()
		let format = AddrDescriptorFormat::BytesFromIndex;

		let (plist, glist) = match list {
			DefectList::Primary => (true,  false),
			DefectList::Grown   => (false, true),
			DefectList::Both    => (true,  true),
		};

		info!("issuing READ DEFECT DATA(12): plist={:?} glist={:?} format={:?}", plist, glist, format);

		let plist = if plist { 1 } else { 0 };
		let glist = if glist { 1 } else { 0 };

		// we're only interested in the header, not the list itself
		const alloc: usize = 8;

		let cmd: [u8; 12] = [
			0xb7, // opcode
			(plist << 4) + (glist << 3) + (format as u8), // reserved (3 bits), req_plist, req_glist, defect list format (3 bits)
			0, 0, 0, 0, // reserved
			((alloc >> 24) & 0xff) as u8,
			((alloc >> 16) & 0xff) as u8,
			((alloc >>  8) & 0xff) as u8,
			( alloc        & 0xff) as u8,
			0, // reserved
			0, // control (XXX what's that?!)
		];

		let (sense, data) = self.do_cmd(&cmd, Direction::From, 32, alloc)?;

		if sense.len() > 0 {
			// only current senses are expected here
			if let Some((true, sense)) = sense::parse(&sense) {
				match sense.kcq() {
					// DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x00)) => return Ok(Some(0)),
					// PRIMARY DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x01)) |
					// GROWN DEFECT LIST NOT FOUND
					Some((_, 0x1c, 0x02)) => {
						if list != DefectList::Both {
							return Ok(Some(0))
						} // else fall through to the data parser
						// XXX is it correct to just dismiss (WHATEVER) DEFECT LIST NOT FOUND if DefectList::Both is requested?
					},
					// unexpected sense
					s => return Err(Error::from_sense(&sense)),
				}
			}
		}

		if let Some((format, glistv, plistv, len)) = parse_defect_data_12(&data) {
			debug!("defect list: format={} glistv={} plistv={} len={}\n", format, glistv, plistv, len);

			match (list, plistv, glistv) {
				(DefectList::Primary, true,  false) => (),
				(DefectList::Grown,   false, true)  => (),
				(DefectList::Both,    _,     _)     => (), // XXX match true/true?
				_ => {
					info!("device returned unexpected defect list");
					return Ok(None);
				},
			}

			// see SBC-3, 5.2.2.4
			let entry_size = match format {
				0b000 => 4, // ShortBlock
				0b011 => 8, // LongBlock
				0b100 => 8, // BytesFromIndex
				0b101 => 8, // PhysSector
				_ => {
					info!("unexpected defect list entry format");
					return Ok(None);
				},
			};

			return Ok(Some(len / entry_size));
		} else {
			info!("defect list: not enough data");
			return Ok(None);
		}
	}

	// TODO? struct as a single argument, or maybe even resort to the builder pattern
	/**
	Executes LOG SENSE command.

	Arguments are:

	- `changed`: whether to return code values changed since the last LOG SELECT or LOG CHANGE command (obsolete)
	- `save_params`: record log parameters marked as saveable into non-volatile, vendor-specific location (might not be supported)
	- `default`: whether to return current or default values (?)
	- `threshold`: whether to return cumulative or threshold values
	- `page`, `subpage`: log page to return parameters from
	- `param_ptr`: limit list of return values to parameters starting with id `param_ptr`
	*/
	fn log_sense(&self, changed: bool, save_params: bool, default: bool, threshold: bool, page: u8, subpage: u8, param_ptr: u16) -> Result<(Vec<u8>, Vec<u8>), Error> {
		info!("issuing LOG SENSE: page={page:?} subpage={subpage:?} param_ptr={param_ptr:?} changed={changed:?} save_params={save_params:?} default={default:?} threshold={threshold:?}",
			changed = changed,
			save_params = save_params,
			default = default,
			threshold = threshold,
			page = page,
			subpage = subpage,
			param_ptr = param_ptr,
		);

		// TODO as u16 argument, not const
		const alloc: usize = 4096;

		// Page Control field
		let pc = match (default, threshold) {
			(false, true) => 0b00, // > threshold values
			(false, false) => 0b01, // > cumulative values
			(true, true) => 0b10, // > default threshold values
			(true, false) => 0b11, // > default cumulative values
		};

		let cmd: [u8; 10] = [
			0x4d, // opcode
			if changed {0b10} else {0} + if save_params {0b1} else {0}, // [reserved × 6][ppc][sp]
			// TODO Err() if page >= 0b1'000'000
			(pc << 6) + page,
			subpage,
			0, // reserved
			(param_ptr >> 8) as u8,
			(param_ptr & 0xff) as u8,
			(alloc >> 8) as u8,
			(alloc & 0xff) as u8,
			0, // control (XXX what's that?!)
		];

		Ok(self.do_cmd(&cmd, Direction::From, 32, alloc)?)
	}

	fn ata_pass_through_16(&self, dir: Direction, regs: &ata::RegistersWrite) -> Result<(ata::RegistersRead, Vec<u8>), ATAError> {
		info!("issuing ATA PASS-THROUGH (16): dir={:?} regs={:?}", dir, regs);

		// see T10/04-262r8a ATA Command Pass-Through, 3.2.3
		let extend = 0; // TODO
		let protocol = match dir {
			Direction::None => 3, // Non-data
			Direction::From => 4, // PIO Data-In
			Direction::To => unimplemented!(), //5, // PIO Data-Out
			_ => unimplemented!(),
		};
		let multiple_count = 0; // TODO
		let ata_cmd: [u8; 16] = [
			0x85, // opcode: ATA PASS-THROUGH (16)
			(multiple_count << 5) + (protocol << 1) + extend,
			// 0b00: wait up to 2^(OFF_LINE+1)-2 seconds for valid ATA status register
			// 0b1: CK_COND, return ATA register info in the sense data
			// 0b0: reserved
			// 0b1: T_DIR; transfer from ATA device
			// 0b1: BYT_BLOK; T_LENGTH is in blocks, not in bytes
			// 0b01: T_LENGTH itself
			0b0010_1101,
			0, regs.features,
			0, regs.sector_count,
			0, regs.sector,
			0, regs.cyl_low,
			0, regs.cyl_high,
			regs.device,
			regs.command,
			0, // control (XXX what's that?!)
		];

		let (sense, data) = self.do_cmd(&ata_cmd, Direction::From, 32, 512)?;

		let sense = match sense::parse(&sense) {
			Some((true, sense)) => sense,
			Some((false, _)) | None => {
				// no (current) sense
				return Err(ATAError::NoRegisters);
			},
		};

		let descriptors = match sense {
			// current sense in the descriptor format
			sense::Sense::Descriptor(sense::DescriptorData {
				descriptors,
				// Recovered Error / ATA PASS THROUGH INFORMATION AVAILABLE
				key: 0x01, asc: 0x00, ascq: 0x1D,
				..
			}) => {
				descriptors
			},

			sense::Sense::Fixed(sense::FixedData::Valid {
				// Illegal Request / INVALID COMMAND OPERATION CODE
				key: 0x05, asc: 0x20, ascq: 0x00, ..
			}) => {
				return Err(ATAError::NotSupported);
			},

			// unexpected sense
			sense => return Err(Error::from_sense(&sense))?,
		};

		for desc in descriptors {
			if desc.code != 0x09 { continue; }
			if desc.data.len() != 12 { continue; }

			let d = desc.data;

			// TODO? EXTEND bit, ATA PASS-THROUGH 12 vs 16
			return Ok((ata::RegistersRead {
				error: d[1],

				sector_count: d[3],

				sector: d[5],
				cyl_low: d[7],
				cyl_high: d[9],
				device: d[10],

				status: d[11],
			}, data))
		}

		return Err(ATAError::NoRegisters);
	}
}

impl SCSICommon for SCSIDevice {
	// XXX DRY
	fn do_cmd(&self, cmd: &[u8], dir: Direction, sense_len: usize, data_len: usize) -> Result<(Vec<u8>, Vec<u8>), io::Error> {
		Self::do_cmd(self, cmd, dir, sense_len, data_len)
	}
}

// The following return tuple of (format, glistv, plistv, len)
fn parse_defect_data_10(data: &[u8]) -> Option<(u8, bool, bool, u16)> {
	if data.len() >= 4 {
		// byte 0: reserved

		// > A device server unable to return the requested format shall return the defect list in its default format and indicate that format in the DEFECT LIST FORMAT field in the defect list header
		let format = data[1] & 0b111;
		let glistv = data[1] & 0b1000 != 0;
		let plistv = data[1] & 0b10000 != 0;
		// byte 1 bits 5..7: reserved

		let len = (&data[2..4]).read_u16::<BigEndian>().unwrap();

		// the rest is the address list itself

		return Some((format, glistv, plistv, len));
	}

	None
}
fn parse_defect_data_12(data: &[u8]) -> Option<(u8, bool, bool, u32)> {
	if data.len() >= 8 {
		// byte 0: reserved

		// > A device server unable to return the requested format shall return the defect list in its default format and indicate that format in the DEFECT LIST FORMAT field in the defect list header
		let format = data[1] & 0b111;
		let glistv = data[1] & 0b1000 != 0;
		let plistv = data[1] & 0b10000 != 0;
		// byte 1 bits 5..7: reserved

		// bytes 2, 3: reserved

		let len = (&data[4..8]).read_u32::<BigEndian>().unwrap();

		// the rest is the address list itself

		return Some((format, glistv, plistv, len));
	}

	None
}
