extern crate libc;
use std::mem;

use cam::*;

use ata;
use ata::ATADevice;
use Direction;
use Device;

use std::io::Error;

impl ATADevice for Device {
	fn ata_do(&self, dir: Direction, regs: &ata::RegistersWrite) -> Result<(ata::RegistersRead, [u8; 512]), Error> {
		let timeout = 10; // in seconds; TODO configurable

		let mut data: [u8; 512] = [0; 512];

		let ccb = CCB(&mut unsafe { mem::zeroed() } as *mut _);

		unsafe {
			let h = ccb.ccb_h();
			h.func_code = xpt_opcode::XPT_ATA_IO;
			h.flags = match dir {
				Direction::From => ccb_flags::CAM_DIR_IN,
				Direction::To => ccb_flags::CAM_DIR_OUT,
				Direction::Both => ccb_flags::CAM_DIR_BOTH,
				Direction::None => ccb_flags::CAM_DIR_NONE,
			} as u32;
			h.retry_count = 0;
			h.timeout = timeout * 1000;

			let ataio = ccb.ataio();
			ataio.data_ptr = data.as_mut_ptr();
			ataio.dxfer_len = 512;
			ataio.ata_flags = 0;

			ataio.cmd.command	= regs.command;
			ataio.cmd.features	= regs.features;
			ataio.cmd.lba_low_exp	= 0;
			ataio.cmd.lba_low	= regs.sector;
			ataio.cmd.lba_mid_exp	= 0;
			ataio.cmd.lba_mid	= regs.cyl_low;
			ataio.cmd.lba_high_exp	= 0;
			ataio.cmd.lba_high	= regs.cyl_high;
			ataio.cmd.device	= regs.device;
			ataio.cmd.sector_count	= regs.sector_count;

			ataio.cmd.flags = (CAM_ATAIO_NEEDRESULT | CAM_ATAIO_48BIT) as u8;

			h.flags |= ccb_flags::CAM_DEV_QFRZDIS as u32;
		}

		self.dev.send_ccb(&ccb)?;

		if ccb.get_status() != (cam_status::CAM_REQ_CMP as u32) {
			Err(CAMError::current())?
		}

		let ataio = unsafe { ccb.ataio() };

		Ok((ata::RegistersRead {
			error: ataio.res.error,

			sector_count: ataio.res.sector_count,

			sector: ataio.res.lba_low,
			cyl_low: ataio.res.lba_mid,
			cyl_high: ataio.res.lba_high,
			device: ataio.res.device,

			status: ataio.res.status,
		}, data))
	}
}
