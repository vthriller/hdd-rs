//! Thin wrapper against FreeBSD's libcam.
//!
//! This module is not intended to be a full-featured wrapper even for a subset of all the things that libcam provides.
//! Instead, this module offers a number of helpers and shortcuts, like `impl Drop`, that should aid with writing a bit more concise and idiomatic code against libcam.
//! Users of this module are expected to do most of the things on their own, manually, using things like `unsafe {}` and `.0`.
//!
//! This module is not for a general use. Besides the fact that it is utterly incomplete and unfriendly, this binding also lacks a lot of things that libcam provides, as they are irrelevant to the purposes of the crate.
//! For the list of exported FFI interfaces, consult `cam/build.rs`.
//!
//! For more on CAM, see [cam(4)][man-cam], [FreeBSD Architecture Handbook][arch-handbook-scsi], [The Design and Implementation of the FreeBSD SCSI Subsystem][difss], or just lurk around [source files in e.g. /usr/src/sbin/camcontrol/][camcontrol-svn].
//!
//! [man-cam]: https://www.freebsd.org/cgi/man.cgi?query=cam&apropos=0&sektion=4&manpath=FreeBSD+11.1-RELEASE+and+Ports&arch=default&format=html
//! [arch-handbook-scsi]: https://www.freebsd.org/doc/en_US.ISO8859-1/books/arch-handbook/scsi.html
//! [difss]: https://people.freebsd.org/~gibbs/ARTICLE-0001.html
//! [camcontrol-svn]: https://svnweb.freebsd.org/base/stable/11/sbin/camcontrol/

pub mod bindings;
pub use bindings::{
	CAM_ATAIO_48BIT,
	CAM_ATAIO_NEEDRESULT,
	MSG_SIMPLE_Q_TAG,
	cam_status,
	ccb_flags,
	xpt_opcode,
};

pub mod device;
pub use device::*;
pub mod ccb;
pub use ccb::*;
pub mod error;
pub use error::*;
