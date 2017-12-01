use cam::{CAMDevice, CCB, error};
use cam::bindings::{xpt_opcode, cam_status, cam_proto};
use std::io;

/// See [parent module docs](../index.html)
#[derive(Debug)]
pub struct Device {
	pub dev: CAMDevice,
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
