pub mod data;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

use std::io::{Error, ErrorKind};
use ata;
use byteorder::{ReadBytesExt, BigEndian};
use self::data::sense;

use Direction;

// TODO reindent
pub trait SCSIDevice {

/// Executes `cmd` and puts response in the `buf`. Returns SCSI sense.
fn do_cmd(&self, cmd: &[u8], dir: Direction, buf: &mut [u8])-> Result<[u8; 64], Error>;

fn scsi_inquiry(&self, vital: bool, code: u8) -> Result<([u8; 64], [u8; 4096]), Error> {
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
	let mut buf = [0u8; alloc];

	let sense = self.do_cmd(&cmd, Direction::From, &mut buf)?;

	Ok((sense, buf))
}

/// returns tuple of (sense, logical block address, block length in bytes)
fn read_capacity_10(&self, lba: Option<u32>) -> Result<([u8; 64], u32, u32), Error> {
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
	let mut buf = [0u8; 8];

	let sense = self.do_cmd(&cmd, Direction::From, &mut buf)?;

	Ok((
		sense,
		(&buf[0..4]).read_u32::<BigEndian>().unwrap(),
		(&buf[4..8]).read_u32::<BigEndian>().unwrap(),
	))
}

// TODO? struct as a single argument
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
fn log_sense(&self, changed: bool, save_params: bool, default: bool, threshold: bool, page: u8, subpage: u8, param_ptr: u16) -> Result<([u8; 64], [u8; 4096]), Error> {
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
	let mut buf = [0u8; alloc];

	let sense = self.do_cmd(&cmd, Direction::From, &mut buf)?;

	Ok((sense, buf))
}

fn ata_pass_through_16(&self, dir: Direction, regs: &ata::RegistersWrite) -> Result<(ata::RegistersRead, [u8; 512]), Error> {
	// see T10/04-262r8a ATA Command Pass-Through, 3.2.3
	let extend = 0; // TODO
	let protocol = match dir {
		Direction::None => 3, // Non-data
		Direction::From => 4, // PIO Data-In
		Direction::To => 5, // PIO Data-Out
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

	let mut buf: [u8; 512] = [0; 512];

	let sense = self.do_cmd(&ata_cmd, Direction::From, &mut buf)?;

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

		let data = desc.data;

		// TODO? EXTEND bit, ATA PASS-THROUGH 12 vs 16
		return Ok((ata::RegistersRead {
			error: data[1],

			sector_count: data[3],

			sector: data[5],
			cyl_low: data[7],
			cyl_high: data[9],
			device: data[10],

			status: data[11],
		}, buf))
	}

	// TODO proper error
	return Err(Error::new(ErrorKind::Other, "no (valid) sense descriptors found"));
}

}
