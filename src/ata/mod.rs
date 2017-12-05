/*!
All things ATA.

* Import [`ATADevice`](struct.ATADevice.html) to start sending ATA commands to the [`Device`](../device/index.html) or [`SCSIDevice`](../scsi/index.html).
* Use [`data` module](data/index.html) to parse various low-level structures found in ATA command replies.
* Import traits from porcelain modules (currently that's just [`misc`](misc/index.html)) to do typical tasks without needing to compose commands and parse responses yourself.
*/

pub mod data;
pub mod misc;

use Direction;
use scsi::{self, SCSIDevice, SCSICommon};

#[derive(Debug, Clone, Copy)]
pub enum Command {
	Identify = 0xec,
	SMART = 0xb0,
}
#[derive(Debug, Clone, Copy)]
pub enum SMARTFeature {
	ReadValues = 0xd0, // in ATA8-ACS it's called 'SMART READ DATA', which is a bit unclear to people not familiar with ATA… or sometimes even to some who knows ATA well
	ReadThresholds = 0xd1,
	ReturnStatus = 0xda,
}

// data port is omitted for obvious reasons
#[derive(Debug)]
pub struct RegistersRead {
	pub error: u8,

	pub sector_count: u8,

	pub sector: u8, // lba (least significant bits)
	pub cyl_low: u8, // lba
	pub cyl_high: u8, // lba
	pub device: u8, // lba (most significant bits); aka drive/head, device/head, select

	pub status: u8,
}
#[derive(Debug)]
pub struct RegistersWrite {
	pub features: u8,

	pub sector_count: u8,

	pub sector: u8,
	pub cyl_low: u8,
	pub cyl_high: u8,
	pub device: u8,

	pub command: u8,
}

#[derive(Debug)]
pub struct ATADevice<T> {
	device: T,
}

impl<T> ATADevice<T> {
	pub fn new(device: T) -> Self {
		Self { device }
	}
}

// using macro here instead of trait because we don't want to define yet another trait just to declare that `T` in `ATADevice<T>` is capable of `ata_platform_do()`,
// which is an implementation detail that would leak everywhere as part of a public interface
// besides, we really only need this method for just, like, two types: `ATADevice<Device>` and `ATADevice<SCSIDevice>`
macro_rules! ata_do { ($Err:ty) => {
	pub fn ata_do(&self, dir: Direction, regs: &::ata::RegistersWrite) -> Result<(::ata::RegistersRead, Vec<u8>), $Err> {
		info!("issuing cmd: dir={:?} regs={:?}", dir, regs);

		// this one is implemented in `mod {linux,freebsd}`, and here for `T: SCSIDevice`
		let ret = Self::ata_platform_do(self, dir, regs);
		match ret {
			Ok((ref regs, ref data)) => {
				debug!("cmd reply: regs={:?}", regs);
				// XXX does it make sense to use hexdump_16() instead of hexdump_8() if cmd is not IDENTIFY DEVICE?
				debug!("cmd data: {}", ::utils::hexdump_16(&::utils::bytes_to_words(data)));
			},
			ref err => {
				debug!("cmd error: {:?}", err);
			},
		}
		ret
	}
} }

/*
One might notice there's no linux support here. There's a couple of reasons for that:
- generally available ioctls like HDIO_DRIVE_{CMD,TASK} are too specialized and unsuitable for writing generic code
- more generic ioctl, HDIO_DRIVE_TASKFILE is not available if kernel is not built with CONFIG_IDE_TASK_IOCTL (and it is indeed absent in modern mainstream distros)
- all HDIO_* ioctls are full of quirks like conditionally pre-filled and masked registers (see Documentation/ioctl/hdio.txt)
- CONFIG_IDE is disabled for a really long time in modern distros, and support for most of HDIO_* ioctls is absent from libata in favour of issuing ATA commangs through SG_IO, which is already covered in scsi module of this crate
*/

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;

impl ATADevice<SCSIDevice> {
	ata_do!(scsi::ATAError);
	fn ata_platform_do(&self, dir: Direction, regs: &RegistersWrite) -> Result<(RegistersRead, Vec<u8>), scsi::ATAError> {
		self.device.ata_pass_through_16(dir, regs)
	}

	/// Return the wrapped device. Useful in cases when ATA PASS-THROUGH is used to determine whether this is an ATA device or not.
	pub fn unwrap(self) -> SCSIDevice {
		self.device
	}
}
