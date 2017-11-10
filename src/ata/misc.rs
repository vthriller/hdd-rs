use Direction;

use ata::{ATADevice, RegistersRead, RegistersWrite, Command, SMARTFeature};
use scsi::SCSIDevice;

use ata::data::{id, health, attr};
use drivedb;

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

	fn get_smart_health(&self) -> Result<Option<bool>, Error> {
		let (regs, _) = self.ata_do(Direction::None, &RegistersWrite {
			command: Command::SMART as u8,
			features: SMARTFeature::ReturnStatus as u8,
			sector_count: 0,
			sector: 0,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		})?;
		Ok(health::parse_smart_status(&regs))
	}

	fn get_smart_attributes(&self, dbentry: &Option<drivedb::Match>) -> Result<Vec<attr::SmartAttribute>, Error> {
		let (_, data) = self.ata_do(Direction::From, &RegistersWrite {
			command: Command::SMART as u8,
			sector: 0,
			features: SMARTFeature::ReadValues as u8,
			sector_count: 1,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		})?;
		let (_, thresh) = self.ata_do(Direction::From, &RegistersWrite {
			command: Command::SMART as u8,
			sector: 0,
			features: SMARTFeature::ReadThresholds as u8,
			sector_count: 1,
			cyl_low: 0x4f,
			cyl_high: 0xc2,
			device: 0,
		})?;

		Ok(attr::parse_smart_values(&data, &thresh, &dbentry))
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
