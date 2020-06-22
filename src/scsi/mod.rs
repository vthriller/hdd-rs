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
			// no from() here, as SCSI sense is also used for informational purposes
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

		// FIXME this block is full of super-awkward patterns
		let descriptors = match sense::parse(&sense) {
			// current sense in the descriptor format
			Some((true, sense::Sense::Descriptor(sense::DescriptorData {
				descriptors,
				// Recovered Error / ATA PASS THROUGH INFORMATION AVAILABLE
				key: 0x01, asc: 0x00, ascq: 0x1D,
				..
			}))) => {
				descriptors
			},

			Some((true, sense::Sense::Descriptor(sense::DescriptorData {
				descriptors,
				// some devices/drivers return (Ok, 0, 0) as a sense;
				// will validate its contents below
				key: 0x00, asc: 0x00, ascq: 0x00,
				..
			}))) => {
				descriptors
			},

			Some((true, sense::Sense::Fixed(sense::FixedData::Valid {
				// Illegal Request / INVALID COMMAND OPERATION CODE
				key: 0x05, asc: 0x20, ascq: 0x00, ..
			}))) => {
				return Err(ATAError::NotSupported);
			},

			Some((true, sense::Sense::Fixed(sense::FixedData::Valid {
				key, asc, ascq, ..
			})))
			| Some((true, sense::Sense::Descriptor(sense::DescriptorData {
				key, asc, ascq, ..
			})))
			=> {
				// unexpected sense
				return Err(Error::Sense(sense::key::SenseKey::from(key), asc, ascq))?;
			},

			Some((true, sense::Sense::Fixed(sense::FixedData::Invalid(_)))) => {
				// invalid sense
				return Err(Error::Nonsense)?;
			},

			Some((false, _)) | None => {
				// no (current) sense
				return Err(ATAError::NoRegisters);
			},
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
