#[cfg(target_os = "linux")]
use linux_scsi::do_cmd;

#[cfg(target_os = "freebsd")]
use freebsd_scsi::do_cmd;

use std::io::{Error, ErrorKind};
use ata;
use byteorder::{ReadBytesExt, BigEndian};

pub fn scsi_inquiry(file: &str, vital: bool, code: u8) -> Result<([u8; 64], [u8; 4096]), Error> {
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

	let sense = do_cmd(file, &cmd, &mut buf)?;

	Ok((sense, buf))
}

/// returns tuple of (sense, logical block address, block length in bytes)
pub fn read_capacity_10(file: &str, lba: Option<u32>) -> Result<([u8; 64], u32, u32), Error> {
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

	let sense = do_cmd(file, &cmd, &mut buf)?;

	Ok((
		sense,
		(&buf[0..4]).read_u32::<BigEndian>().unwrap(),
		(&buf[4..8]).read_u32::<BigEndian>().unwrap(),
	))
}

fn ata_pass_through_16(file: &str, regs: &ata::RegistersWrite) -> Result<([u8; 64], [u8; 512]), Error> {
	// see T10/04-262r8a ATA Command Pass-Through, 3.2.3
	let extend = 0; // TODO
	let protocol = 4; // PIO Data-In; TODO
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

	let sense = do_cmd(file, &ata_cmd, &mut buf)?;

	Ok((sense, buf))
}

pub fn ata_pass_through_16_exec(file: &str, regs: &ata::RegistersWrite) -> Result<[u8; 512], Error> {
	let (_, buf) = ata_pass_through_16(file, regs)?;
	Ok(buf)
}

pub fn ata_pass_through_16_task(file: &str, regs: &ata::RegistersWrite) -> Result<ata::RegistersRead, Error> {
	let (sense, _) = ata_pass_through_16(file, regs)?;

	if sense[0] & 0x7f != 0x72 {
		// we expected current sense in the descriptor format
		// TODO proper error
		return Err(Error::new(ErrorKind::Other, "unexpected sense format/whatever"));
	}

	// sense[7] is the additional sense length; in other words, it's what amount of data descriptors occupy
	let sense_length = 8 + sense[7] as usize;
	let mut current_desc: usize = 8;
	// iterate over descriptors
	while current_desc < sense_length {
		let (code, length) = (sense[current_desc], sense[current_desc + 1]);
		if !(code == 0x09 && length == 12) {
			// descriptor is not about ATA Status or is malformed (invalid length)
			current_desc += length as usize;
			continue;
		}
		// TODO? EXTEND bit, ATA PASS-THROUGH 12 vs 16
		return Ok(ata::RegistersRead {
			error: sense[current_desc + 3],

			sector_count: sense[current_desc + 5],

			sector: sense[current_desc + 7],
			cyl_low: sense[current_desc + 9],
			cyl_high: sense[current_desc + 11],
			device: sense[current_desc + 12],

			status: sense[current_desc + 13],
		})
	}

	// TODO proper error
	return Err(Error::new(ErrorKind::Other, "no (valid) sense descriptors found"));
}
