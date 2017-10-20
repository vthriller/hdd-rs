pub enum Command {
	Identify = 0xec,
	SMART = 0xb0,
}
pub enum SMARTFeature {
	ReadValues = 0xd0, // in ATA8-ACS it's called 'SMART READ DATA', which is a bit unclear to people not familiar with ATA… or sometimes even to some who knows ATA well
	ReadThresholds = 0xd1,
	ReturnStatus = 0xda,
}

#[cfg(target_os = "linux")]
use linux_ata;
#[cfg(target_os = "linux")]
pub use self::linux_ata::*;

#[cfg(target_os = "freebsd")]
use freebsd_ata;
#[cfg(target_os = "freebsd")]
pub use self::freebsd_ata::*;
