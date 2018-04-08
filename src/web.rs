
use iron::{Iron, IronResult, Listening, status};
use iron::error::HttpResult;
use iron::response::Response;
#[cfg(feature = "gzip")]
use iron::response::WriteBody;
use iron::request::Request;
use iron::middleware::Handler;
use iron::mime::Mime;

use sysinfo::{System, SystemExt};

#[cfg(feature = "gzip")]
use flate2::Compression;
#[cfg(feature = "gzip")]
use flate2::write::GzEncoder;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
#[cfg(feature = "gzip")]
use std::io::{self, Write};

use SysinfoExt;
use serde_json;


const INDEX_HTML: &'static [u8] = include_bytes!("index.html");
const FAVICON: &'static [u8] = include_bytes!("../resources/favicon.ico");
const REFRESH_DELAY: u64 = 60 * 10; // 10 minutes

/// Simple wrapper to get gzip compressed output on string types.
#[cfg(feature = "gzip")]
struct GzipContent(Box<WriteBody>);

#[cfg(feature = "gzip")]
impl WriteBody for GzipContent {
    fn write_body(&mut self, w: &mut Write) -> io::Result<()> {
        let mut w = GzEncoder::new(w, Compression::default());
        self.0.write_body(&mut w)?;
        w.finish().map(|_| ())
    }
}

struct SysinfoIronHandler(Arc<DataHandler>);

struct DataHandler {
    system: RwLock<System>,
    last_connection: Mutex<SystemTime>,
    json_output: RwLock<String>,
}

impl DataHandler {
    fn can_update_system_info(&self) -> bool {
        SystemTime::now().duration_since(*self.last_connection.lock().unwrap())
                         .unwrap()
                         .as_secs() < REFRESH_DELAY
    }

    fn update_last_connection(&self) {
        *self.last_connection.lock().unwrap() = SystemTime::now();
    }
}

#[cfg(feature = "gzip")]
macro_rules! return_gzip_or_not {
    ($req:expr, $content:expr, $typ:expr) => {{
        let mut use_gzip = false;

        if let Some(raw_accept_encoding) = $req.headers.get_raw("accept-encoding") {
            for accept_encoding in raw_accept_encoding {
                match ::std::str::from_utf8(accept_encoding).map(|s| s.to_lowercase()) {
                    Ok(ref s) if s.contains("gzip") => {
                        use_gzip = true;
                        break;
                    }
                    _ => continue,
                }
            }
        }
        if !use_gzip {
            Ok(Response::with((status::Ok, $typ.parse::<Mime>().unwrap(), $content)))
        } else {
            let mut res = Response::new();
            res.status = Some(status::Ok);
            res.body = Some(Box::new(GzipContent(Box::new($content))));
            res.headers.append_raw("content-type", $typ.as_bytes().to_vec());
            res.headers.append_raw("content-encoding", vec![b'g', b'z', b'i', b'p']);
            Ok(res)
        }
    }}
}

#[cfg(not(feature = "gzip"))]
macro_rules! return_gzip_or_not {
    ($req:expr, $content:expr, $typ:expr) => {{
        Ok(Response::with((status::Ok, $typ.parse::<Mime>().unwrap(), $content)))
    }}
}

impl Handler for SysinfoIronHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match match req.url.path().last() {
            Some(path) => {
                if *path == "" {
                    1
                } else if *path == "favicon.ico" {
                    2
                } else {
                    3
                }
            }
            None => 0,
        } {
            1 => return_gzip_or_not!(req, INDEX_HTML, "text/html"),
            2 => return_gzip_or_not!(req, FAVICON, "image/x-icon"),
            3 => {
                self.0.update_last_connection();
                return_gzip_or_not!(req,
                                    self.0.json_output.read().unwrap().clone(),
                                    "application/json")
            }
            _ => Ok(Response::with((status::NotFound, "Not found"))),
        }
    }
}

pub fn start_web_server(sock_addr: Option<String>) -> HttpResult<Listening> {
    let data_handler = Arc::new(DataHandler {
        system: RwLock::new(System::new()),
        last_connection: Mutex::new(SystemTime::now()),
        json_output: RwLock::new(String::from("[]")),
    });
    let data_handler_clone = data_handler.clone();
    thread::spawn(move || {
        let mut sleeping = false;
        loop {
            if data_handler_clone.can_update_system_info() {
                {
                    let mut system = data_handler_clone.system.write().unwrap();
                    system.refresh_all();
                    // refresh it twice to provide accurate information after wake up
                    if sleeping {
                        system.refresh_all();
                        sleeping = false;
                    }
                    let sysinfo = SysinfoExt::new(&system);
                    let mut json_output = data_handler_clone.json_output.write().unwrap();
                    json_output.clear();
                    use std::fmt::Write;
                    json_output.write_str(&serde_json::to_string(&sysinfo)
                                          .unwrap_or(String::from("[]"))).unwrap();
                }
                thread::sleep(Duration::new(5, 0));
            } else {
                // If we don't need to refresh the system information, we can sleep a lot less.
                thread::sleep(Duration::from_millis(500));
                sleeping = true;
            }
        }
    });
    let mut iron = Iron::new(SysinfoIronHandler(data_handler));
    iron.threads = 4;
    iron.http(sock_addr.unwrap_or("localhost:3000".to_owned()))
}
