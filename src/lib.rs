//! Lightweight web based process viewer built on top of 
//! [sysinfo](https://github.com/GuillaumeGomez/sysinfo).
//! [See more info in GitHub repository](https://github.com/onur/sysinfo-web).

pub extern crate sysinfo;
extern crate serde_json;
extern crate serde;
extern crate hostname;
#[cfg(feature = "gzip")]
extern crate flate2;
extern crate warp;

pub mod sysinfo_serde;
mod sysinfo_ext;
mod web;

pub use sysinfo_ext::SysinfoExt;
pub use web::start_web_server;
