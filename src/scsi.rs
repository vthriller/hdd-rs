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
pub fn ata_pass_through_16_exec(file: &str, cmd: ata::Command, sector: u8, feature: u8, nsector: u8) -> Result<[u8; 512], Error> {
	unimplemented!()
}
#[cfg(target_os = "freebsd")]
#[allow(unused_variables)]
pub fn ata_pass_through_16_task(file: &str, cmd: ata::Command, feature: u8, nsector: u8, sector: u8, lcyl: u8, hcyl: u8, _: u8) -> Result<[u8; 7], Error> {
	unimplemented!()
}
