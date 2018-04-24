#![allow(
	non_upper_case_globals,
	non_camel_case_types,
	non_snake_case,
	dead_code,

	missing_debug_implementations,
	missing_docs,
	missing_copy_implementations,
	trivial_casts,
	trivial_numeric_casts,
	unsafe_code,
	unstable_features,
	unused_import_braces,
	unused_qualifications,
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/*
XXX bindgen can't currently handle `#define` in the form of `(~(whatever)0)`
see also https://github.com/rust-lang-nursery/rust-bindgen/issues/316

re: `bindgen-freebsd.h`:
- bindgen turns `const …` into `pub static` declaration, which is not what we're looking for,
- no need to `#undef`s anything, we just don't whitelist vars in the `build.rs` (also, `#undef` doesn't seem to work anyways).
*/

// #define CAM_XPT_PATH_ID ((path_id_t)~0)
pub const CAM_XPT_PATH_ID: path_id_t = !0;
// #define CAM_BUS_WILDCARD ((path_id_t)~0)
pub const CAM_BUS_WILDCARD: path_id_t = !0;
// #define CAM_TARGET_WILDCARD ((target_id_t)~0)
pub const CAM_TARGET_WILDCARD: target_id_t = !0;
// #define CAM_LUN_WILDCARD (~(u_int)0)
pub const CAM_LUN_WILDCARD: u_int = !0;

/*
similarly,

/usr/include/cam/scsi/scsi_pass.h
39:#define CAMIOCOMMAND _IOWR(CAM_VERSION, 2, union ccb)

/usr/include/cam/cam_ccb.h
574:#define CAM_VERSION 0x19    /* Hex value for current version */

/usr/include/sys/ioccom.h
61:#define      _IOWR(g,n,t)    _IOC(IOC_INOUT, (g), (n), sizeof(t))

/usr/include/sys/ioccom.h
51:#define      IOC_INOUT       (IOC_IN|IOC_OUT)

/usr/include/sys/ioccom.h
49:#define      IOC_OUT         0x40000000      /* copy out parameters */
50:#define      IOC_IN          0x80000000      /* copy in parameters */

/usr/include/sys/ioccom.h
54:#define      _IOC(inout,group,num,len)       ((unsigned long) \
55-     ((inout) | (((len) & IOCPARM_MASK) << 16) | ((group) << 8) | (num)))

/usr/include/sys/ioccom.h
42:#define      IOCPARM_MASK    ((1 << IOCPARM_SHIFT) - 1) /* parameter length mask */

/usr/include/sys/ioccom.h
41:#define      IOCPARM_SHIFT   13              /* number of bits for ioctl size */
*/

use std::os::raw::c_ulong;
use std::mem::size_of;

const IOCPARM_MASK: c_ulong = ((1 << 13) - 1);
const IOC_OUT: c_ulong = 0x40000000;
const IOC_IN:  c_ulong = 0x80000000;
pub const CAMIOCOMMAND: c_ulong = IOC_IN | IOC_OUT | ((size_of::<ccb>() as c_ulong & IOCPARM_MASK) << 16) | (0x19 << 8) | 2;
