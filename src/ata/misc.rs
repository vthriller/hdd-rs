use Direction;

use ata::{ATADevice, RegistersRead, RegistersWrite, Command};
use scsi::SCSIDevice;

use ata::data::id;

use std::io::Error;

// TODO proper errors
pub trait Misc {
	// FIXME? put this into {SCSI,ATA}Device under some consistent name
	fn ata_do(&self, dir: Direction, regs: &RegistersWrite) -> Result<(RegistersRead, Vec<u8>), Error>;

	fn get_device_id(&self) -> Result<id::Id, Error> {
		let (_, data) = self.ata_do(Direction::From, &RegistersWrite {
			command: Command::Identify as u8,
			sector: 1,
			features: 0,
			sector_count: 1,
			cyl_high: 0,
			cyl_low: 0,
			device: 0,
		})?;

		Ok(id::parse_id(&data))
	}
}

impl Misc for ATADevice {
	fn ata_do(&self, dir: Direction, regs: &RegistersWrite) -> Result<(RegistersRead, Vec<u8>), Error> {
		ATADevice::ata_do(self, dir, regs)
	}
}

impl Misc for SCSIDevice {
	fn ata_do(&self, dir: Direction, regs: &RegistersWrite) -> Result<(RegistersRead, Vec<u8>), Error> {
		self.ata_pass_through_16(dir, regs)
	}
}
