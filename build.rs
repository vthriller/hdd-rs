// XXX seems like Cargo has no support for target-specific build scripts…

#[cfg(target_os = "freebsd")]
extern crate bindgen;

#[cfg(target_os = "freebsd")]
use std::env;
#[cfg(target_os = "freebsd")]
use std::path::PathBuf;

#[cfg(target_os = "freebsd")]
fn main() {
	println!("cargo:rustc-link-lib=cam");

	let bindings = bindgen::Builder::default()
		.header("wrapper.h")
		.whitelisted_function("cam_(open|close)_device")
		.whitelisted_function("cam_(get|free)ccb")
		.whitelisted_function("cam_send_ccb")
		.whitelisted_type("ccb_flags")
		.whitelisted_type("cam_status")
		.whitelisted_var("cam_errbuf")
		.whitelisted_var("CAM_ATAIO_.*")
		.whitelisted_var("MSG_SIMPLE_Q_TAG")
		.generate()
		.expect("Unable to generate bindings");

	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}

#[cfg(not(target_os = "freebsd"))]
fn main() {
}
