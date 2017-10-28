pub enum Command {
	Identify = 0xec,
	SMART = 0xb0,
}
pub enum SMARTFeature {
	ReadValues = 0xd0, // in ATA8-ACS it's called 'SMART READ DATA', which is a bit unclear to people not familiar with ATA… or sometimes even to some who knows ATA well
	ReadThresholds = 0xd1,
	ReturnStatus = 0xda,
}

// data port is omitted for obvious reasons
pub struct RegistersRead {
	pub error: u8,

	pub sector_count: u8,

	pub sector: u8, // lba (least significant bits)
	pub cyl_low: u8, // lba
	pub cyl_high: u8, // lba
	pub device: u8, // lba (most significant bits); aka drive/head, device/head, select

	pub status: u8,
}
pub struct RegistersWrite {
	pub features: u8,

	pub sector_count: u8,

	pub sector: u8,
	pub cyl_low: u8,
	pub cyl_high: u8,
	pub device: u8,

	pub command: u8,
}

/*
One might notice there's no linux support here. There's a couple of reasons for that:
- generally available ioctls like HDIO_DRIVE_{CMD,TASK} are too specialized and unsuitable for writing generic code
- more generic ioctl, HDIO_DRIVE_TASKFILE is not available if kernel is not built with CONFIG_IDE_TASK_IOCTL (and it is indeed absent in modern mainstream distros)
- all HDIO_* ioctls are full of quirks like conditionally pre-filled and masked registers (see Documentation/ioctl/hdio.txt)
- CONFIG_IDE is disabled for a really long time in modern distros, and support for most of HDIO_* ioctls is absent from libata in favour of issuing ATA commangs through SG_IO, which is already covered in scsi module of this crate
*/

use std::io::Error;

#[cfg(target_os = "linux")]
#[allow(unused_variables)]
pub fn ata_do(file: &str, regs: &RegistersWrite) -> Result<(RegistersRead, [u8; 512]), Error> {
	unimplemented!()
}

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
