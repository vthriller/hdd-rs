/*!
Thin wrapper for platform-specific device handle.

This module (and struct it provides) allows opening (`Device::open(&path)`) and interacting with (via [`ata::ATADevice`](../ata/trait.ATADevice.html)/[`scsi::SCSIDevice`](../scsi/trait.SCSIDevice.html) traits) devices in a cross-platform manner, as different operating systems provide different device handles to execute commands against (i.e. regular file descriptor on Linux, `struct cam_device *` on FreeBSD).

## Example

```
use hdd::Device;
use hdd::scsi::SCSIDevice;

let dev = Device::open("/dev/da0").unwrap();
let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();
```
*/

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "freebsd")]
pub mod freebsd;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;
