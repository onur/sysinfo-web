
use std::collections::HashMap;
use std::io::Error;
use serde::{Serialize, Serializer};
use sysinfo::{NetworkExt, System, SystemExt, Disk, Process, ProcessExt, Component, Processor};
use hostname::get_hostname;


/// `sysinfo` with extended features
pub struct SysinfoExt<'a> {
    process_list: HashMap<i32, Process>,
    processor_list: &'a [Processor],
    components_list: &'a [Component],
    disks: &'a [Disk],
    memory: [u64; 6],
    hostname: String,
    uptime: String,
    pub bandwith: (u64, u64),
}


impl<'a> SysinfoExt<'a> {
    pub fn new(system: &'a System) -> Self {
        let network = system.get_network();
        SysinfoExt {
            process_list: system.get_process_list()
                // filter unnamed kernel threads
                .iter().filter(|p| !p.1.name().is_empty())
                .map(|p| (*p.0, p.1.clone()))
                .collect(),
            processor_list: system.get_processor_list(),
            components_list: system.get_components_list(),
            disks: system.get_disks(),
            memory: [system.get_total_memory(),
                     system.get_used_memory(),
                     system.get_free_memory(),
                     system.get_total_swap(),
                     system.get_used_swap(),
                     system.get_free_swap()],
            hostname: get_hostname().unwrap_or("Unknown".to_owned()),
            uptime: get_uptime().unwrap_or("Unknown".to_owned()),
            bandwith: (network.get_income(), network.get_outcome()),
        }
    }
}



impl<'a> Serialize for SysinfoExt<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        use serde::ser::SerializeMap;
        use sysinfo_serde::Ser;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("processor_list",
                             &self.processor_list
                                 .iter()
                                 .map(|p| Ser::new(p))
                                 .collect::<Vec<Ser<Processor>>>())?;
        map.serialize_entry("process_list", &Ser::new(&self.process_list))?;
        map.serialize_entry("components_list",
                             &self.components_list
                                 .iter()
                                 .map(|c| Ser::new(c))
                                 .collect::<Vec<Ser<Component>>>())?;
        map.serialize_entry("disks",
                             &self.disks
                                 .iter()
                                 .map(|d| Ser::new(d))
                                 .collect::<Vec<Ser<Disk>>>())?;
        map.serialize_entry("memory", &self.memory)?;
        map.serialize_entry("hostname", &self.hostname)?;
        map.serialize_entry("uptime", &self.uptime)?;
        map.serialize_entry("bandwith", &self.bandwith)?;
        map.end()
    }
}


/// Gets uptime from /proc/uptime and converts it to human readable String
fn get_uptime() -> Result<String, Error> {
    use std::process::Command;
    let output = Command::new("uptime").output()?;
    Ok(String::from_utf8(output.stdout).unwrap_or("Unknown".to_owned()))
}
