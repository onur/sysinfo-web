
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read};
use std::fs::File;
use std::{thread, time};
use serde::{Serialize, Serializer};
use sysinfo::{System, SystemExt, Disk, Process, Component, Processor};
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
    bandwith: (u64, u64),
}


impl<'a> SysinfoExt<'a> {
    pub fn new(system: &'a System) -> Self {
        SysinfoExt {
            process_list: system.get_process_list()
                .iter()
                // filter unnamed kernel threads
                .filter(|p| !p.1.name.is_empty())
                .map(|p| {
                    (*p.0,
                     {
                         let mut t = p.1.clone();
                         // clear tasks to save up some space
                         #[cfg(target_os = "linux")]
                         t.tasks.clear();
                         // clear environment variables
                         t.environ.clear();
                         // clear command line arguments
                         t.cmd.clear();
                         t
                    })
                })
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
            bandwith: get_bandwith_usage().unwrap_or((0, 0)),
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
        #[cfg(target_os = "linux")]
        map.serialize_entry("show_bandwith_usage", &true)?;
        map.end()
    }
}


/// Gets uptime from /proc/uptime and converts it to human readable String
fn get_uptime() -> Result<String, Error> {
    use std::process::Command;
    let output = Command::new("uptime").output()?;
    Ok(String::from_utf8(output.stdout).unwrap_or("Unknown".to_owned()))
}


#[cfg(target_os = "linux")]
/// Gets bandwith usage
///
/// Returns (Incoming, Outgoing) bytes per seconds
fn get_bandwith_usage() -> Result<(u64, u64), Error> {
    fn read_interface_stat(iface: &str, typ: &str) -> Result<u64, Error> {
        let mut file = File::open(format!("/sys/class/net/{}/statistics/{}_bytes", iface, typ))?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        content.trim().parse()
            .map_err(|_| Error::new(ErrorKind::Other, "Failed to parse network stat"))
    }

    let default_interface = {
        let mut file = File::open("/proc/net/route")?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        content.lines()
            .filter(|l| l.split_whitespace().nth(2).map(|l| l != "00000000").unwrap_or(false))
            .last()
            .and_then(|l| l.split_whitespace().nth(0))
            .ok_or(Error::new(ErrorKind::Other, "Default device not found"))?
            .to_owned()
    };

    let (old_rx, old_tx) = (read_interface_stat(&default_interface, "rx")?,
                            read_interface_stat(&default_interface, "tx")?);
    thread::sleep(time::Duration::from_millis(500));
    let (new_rx, new_tx) = (read_interface_stat(&default_interface, "rx")?,
                            read_interface_stat(&default_interface, "tx")?);

    Ok(((new_rx - old_rx) * 2, (new_tx - old_tx) * 2))
}

#[cfg(not(target_os = "linux"))]
fn get_bandwith_usage() -> Result<(u64, u64), Error> {
    Ok((0, 0))
}
