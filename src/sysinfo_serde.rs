//! Serde serialization support for sysinfo
//!
//! Usage example with `serde_derive`:
//!
//! ```rust
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde_json;
//! # extern crate sysinfo;
//! # extern crate sysinfo_web;
//! # use sysinfo_web::sysinfo_serde;
//! # use sysinfo::SystemExt;
//! # fn main() {
//! #[derive(Serialize)]
//! struct Info (
//!     #[serde(serialize_with = "sysinfo_serde::serialize")]
//!     sysinfo::System
//! );
//!
//! let mut system = sysinfo::System::new();
//! system.refresh_all();
//!
//! let info = Info(system);
//! let serialized = serde_json::to_string(&info).unwrap();
//! # }
//!
//! ```


use std::collections::HashMap;
use serde::{Serialize, Serializer};
use serde::ser::SerializeMap;
use sysinfo::{System, SystemExt, Processor, ProcessorExt, Process, Component, Disk, DiskExt,
              DiskType};


pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
          for<'a> Ser<'a, T>: Serialize
{
    Ser::new(value).serialize(serializer)
}


/// A wrapper to serialize `sysinfo` types.
pub struct Ser<'a, T: 'a>(&'a T);


impl<'a, T> Ser<'a, T>
    where Ser<'a, T>: Serialize
{
    #[inline(always)]
    pub fn new(value: &'a T) -> Self {
        Ser(value)
    }
}


impl<'a> Serialize for Ser<'a, Processor> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("name", self.0.get_name())?;
        map.serialize_entry("cpu_usage", &self.0.get_cpu_usage())?;
        map.end()
    }
}


impl<'a> Serialize for Ser<'a, Process> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("name", &self.0.name)?;
        map.serialize_entry("cmd", &self.0.cmd)?;
        map.serialize_entry("exe", &self.0.exe)?;
        map.serialize_entry("pid", &self.0.pid)?;
        map.serialize_entry("parent", &self.0.parent)?;
        map.serialize_entry("environ", &self.0.environ)?;
        map.serialize_entry("cwd", &self.0.cwd)?;
        map.serialize_entry("root", &self.0.root)?;
        map.serialize_entry("memory", &self.0.memory)?;
        map.serialize_entry("start_time", &self.0.start_time)?;
        map.serialize_entry("cpu_usage", &self.0.cpu_usage)?;
        map.serialize_entry("uid", &self.0.uid)?;
        map.serialize_entry("gid", &self.0.gid)?;
        #[cfg(target_os = "linux")]
        map.serialize_entry("tasks", &Ser::new(&self.0.tasks))?;
        map.end()
    }
}


impl<'a> Serialize for Ser<'a, HashMap<i32, Process>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        for (pid, process) in self.0.iter() {
            map.serialize_key(pid)?;
            map.serialize_value(&Ser::new(process))?;
        }
        map.end()
    }
}


impl<'a> Serialize for Ser<'a, Component> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("temperature", &self.0.temperature)?;
        map.serialize_entry("max", &self.0.max)?;
        map.serialize_entry("critical", &self.0.critical)?;
        map.serialize_entry("label", &self.0.label)?;
        map.end()
    }
}


impl<'a> Serialize for Ser<'a, Disk> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type",
                             &match self.0.get_type() {
                                 DiskType::HDD => "HDD".to_owned(),
                                 DiskType::SSD => "SSD".to_owned(),
                                 DiskType::Unknown(size) => format!("Unknown({})", size),
                             })?;
        map.serialize_entry("name", self.0.get_name().to_str().unwrap())?;
        map.serialize_entry("file_system", ::std::str::from_utf8(self.0.get_file_system()).unwrap())?;
        map.serialize_entry("mount_point", self.0.get_mount_point())?;
        map.serialize_entry("total_space", &self.0.get_total_space())?;
        map.serialize_entry("available_space", &self.0.get_available_space())?;
        map.end()
    }
}


impl<'a> Serialize for Ser<'a, System> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("process_list", &Ser::new(self.0.get_process_list()))?;
        map.serialize_entry("processor_list",
                             &self.0
                                 .get_processor_list()
                                 .iter()
                                 .map(|p| Ser::new(p))
                                 .collect::<Vec<Ser<Processor>>>())?;
        map.serialize_entry("total_memory", &self.0.get_total_memory())?;
        map.serialize_entry("free_memory", &self.0.get_free_memory())?;
        map.serialize_entry("used_memory", &self.0.get_used_memory())?;
        map.serialize_entry("total_swap", &self.0.get_total_swap())?;
        map.serialize_entry("free_swap", &self.0.get_free_swap())?;
        map.serialize_entry("used_swap", &self.0.get_used_swap())?;
        map.serialize_entry("components_list",
                             &self.0
                                 .get_components_list()
                                 .iter()
                                 .map(|c| Ser::new(c))
                                 .collect::<Vec<Ser<Component>>>())?;
        map.serialize_entry("disks",
                             &self.0
                                 .get_disks()
                                 .iter()
                                 .map(|c| Ser::new(c))
                                 .collect::<Vec<Ser<Disk>>>())?;

        map.end()
    }
}
