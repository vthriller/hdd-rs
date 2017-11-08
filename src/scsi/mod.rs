/*!
All things SCSI.

* Import [`SCSIDevice`](trait.SCSIDevice.html) to start sending SCSI commands to the [`Device`](../device/index.html).
* Use [`data` module](data/index.html) to parse various low-level structures found in SCSI replies.
* Import traits from porcelain modules (like [`pages`](pages/index.html)) to do typical tasks without needing to compose commands and parse responses yourself.
*/

pub mod data;
pub mod pages;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

use std::io::{Error, ErrorKind};
use ata;
use byteorder::{ReadBytesExt, BigEndian};
use self::data::sense;

use Direction;

pub trait SCSIDevice {
	/// Executes `cmd` and returns tuple of `(sense, data)`.
	fn do_cmd(&self, cmd: &[u8], dir: Direction, sense_len: u8, data_len: usize) -> Result<(Vec<u8>, Vec<u8>), Error>;

	fn scsi_inquiry(&self, vital: bool, code: u8) -> Result<(Vec<u8>, Vec<u8>), Error> {
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

		self.do_cmd(&cmd, Direction::From, 32, alloc)
	}

	/// returns tuple of (sense, logical block address, block length in bytes)
	fn read_capacity_10(&self, lba: Option<u32>) -> Result<(Vec<u8>, u32, u32), Error> {
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

		self.do_cmd(&cmd, Direction::From, 32, alloc)
	}

	fn ata_pass_through_16(&self, dir: Direction, regs: &ata::RegistersWrite) -> Result<(ata::RegistersRead, Vec<u8>), Error> {
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
			0b00101101,
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

		let descriptors = match sense::parse(&sense) {
			// current sense in the descriptor format
			Some((true, sense::Sense::Descriptor(sense::DescriptorData {
				descriptors, ..
			}))) => {
				descriptors
			},
			_ => {
				// TODO proper error
				return Err(Error::new(ErrorKind::Other, "no (valid) sense descriptors found"));
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

		// TODO proper error
		return Err(Error::new(ErrorKind::Other, "no (valid) sense descriptors found"));
	}
}
