use sysinfo::{System, SystemExt};

#[cfg(feature = "gzip")]
use flate2::Compression;
#[cfg(feature = "gzip")]
use flate2::write::GzEncoder;

use std::io::BufReader;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
#[cfg(feature = "gzip")]
use std::io::{self, Write};
use std::io::prelude::*;

use SysinfoExt;
use serde_json;

use warp::{self, Filter, reject, Rejection, Reply};
use warp::http::{Response, StatusCode};

const INDEX_HTML: &'static [u8] = include_bytes!("index.html");
const FAVICON: &'static [u8] = include_bytes!("../resources/favicon.ico");
const REFRESH_DELAY: u64 = 60 * 10; // 10 minutes
/*
/// Simple wrapper to get gzip compressed output on string types.
#[cfg(feature = "gzip")]
pub struct Gzip;

#[cfg(feature = "gzip")]
impl fairing::Fairing for Gzip {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Gzip compression",
            kind: fairing::Kind::Response,
        }
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        use flate2::{Compression, FlateReadExt};
        use std::io::{Cursor, Read};
        let headers = request.headers();
        if headers
            .get("Accept-Encoding")
            .any(|e| e.to_lowercase().contains("gzip"))
        {
            response.body_bytes().and_then(|body| {
                let mut enc = body.gz_encode(Compression::Default);
                let mut buf = Vec::with_capacity(body.len());
                enc.read_to_end(&mut buf)
                    .map(|_| {
                        response.set_sized_body(Cursor::new(buf));
                        response.set_raw_header("Content-Encoding", "gzip");
                    })
                    .map_err(|e| eprintln!("{}", e)).ok()
            });
        }
    }
}

struct SysinfoIronHandler(Arc<DataHandler>);*/

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
macro_rules! return_gzip_err {
    ($content:expr, $typ:expr, $err:expr) => {{
        eprintln!("Error in gzip compression: {}", $err);
        return Response::builder()
                        .header("content-type", $typ)
                        .body($content.to_owned())
    }}
}

#[cfg(feature = "gzip")]
macro_rules! return_gzip_or_not {
    ($content:expr, $typ:expr) => {{
        warp::header::<String>("accept-encoding")
                     .map(|encoding: String| {
                         let s = encoding.to_lowercase();
                         if s.contains("gzip") || s.contains("*") {
                             let b = BufReader::new($content);
                             let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
                             if gz.write_all($content).is_err() {
                                return_gzip_err!($content, $typ, "write_all failed")
                             }
                             if let Ok(buffer) = gz.finish() {
                                Response::builder()
                                         .header("content-type", $typ)
                                         .header("content-encoding", "gzip")
                                         .body(buffer.to_owned())
                             } else {
                                return_gzip_err!($content, $typ, "finish failed")
                             }
                         } else {
                             Response::builder()
                                      .header("content-type", $typ)
                                      .body($content.to_owned())
                         }
                     })
                     /*.recover(|_| {
                         Ok(Response::builder()
                                  .header("content-type", $typ)
                                  .body($content.to_owned()))
                     })*/
    }}
}

#[cfg(not(feature = "gzip"))]
macro_rules! return_gzip_or_not {
    ($content:expr, $typ:expr) => {{
        Response::builder()
                 .header("content-type", $typ)
                 .body($content)
    }}
}

/*impl Handler for SysinfoIronHandler {
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
}*/

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    match err.status() {
        StatusCode::NOT_FOUND => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("you get a 404, and *you* get a 404..."))
        },
        StatusCode::INTERNAL_SERVER_ERROR => {
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(":fire: this is fine"))
        }
        _ => {
            Err(err)
        }
    }
}

pub fn start_web_server(sock_addr: Option<String>) -> Result<(), ()> {
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
                               .unwrap_or_else(|_| "[]".to_string())).unwrap();
                }
                thread::sleep(Duration::new(5, 0));
            } else {
                // If we don't need to refresh the system information, we can sleep a lot less.
                thread::sleep(Duration::from_millis(500));
                sleeping = true;
            }
        }
    });

    let index = warp::path("favicon.ico").map(|| {
        return_gzip_or_not!(FAVICON, "image/x-icon")
    });
    let update = warp::path("sysinfo.json").map(|| {
        data_handler.update_last_connection();
        return_gzip_or_not!(data_handler.json_output.read().unwrap().clone().as_bytes(), "application/json")
    });
    let index2 = warp::path("").map(|| {
        return_gzip_or_not!(INDEX_HTML, "text/html")
    });

    let routes = warp::get2().and(index/*.or(update).or(index2)*/).recover(customize_error);
    /*let addr = match sock_addr {
        Some(s) => s.split(":")
                    .next()
                    .unwrap_or_else(|| "")
                    .split(".")
                    .filter_map(|s| u32::from_str(s).ok())
                    .collect::<Vec<_>>(),
        None => vec![127, 0, 0, 1],
    };
    if addr.len() != 4 {
        eprintln!("Invalid socket address received");
        return Err(())
    }*/
    warp::serve(routes).run(sock_addr.unwrap_or_else(|| "127.0.0.1".to_owned()));
    Ok(())
}
