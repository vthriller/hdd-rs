#[cfg(target_os = "linux")]
use linux_scsi;
#[cfg(target_os = "linux")]
pub use self::linux_scsi::*;

// TODO
#[cfg(target_os = "freebsd")]
use ata;
#[cfg(target_os = "freebsd")]
use std::io::Error;
#[cfg(target_os = "freebsd")]
#[allow(unused_variables)]
pub fn ata_pass_through_16_exec(file: &str, regs: &ata::RegistersWrite) -> Result<[u8; 512], Error> {
	unimplemented!()
}
#[cfg(target_os = "freebsd")]
#[allow(unused_variables)]
pub fn ata_pass_through_16_task(file: &str, regs: &ata::RegistersWrite) -> Result<ata::RegistersRead, Error> {
	unimplemented!()
}
