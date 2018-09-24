use sysinfo::{System, SystemExt};

#[cfg(feature = "gzip")]
use flate2::Compression;
#[cfg(feature = "gzip")]
use flate2::write::GzEncoder;

use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
#[cfg(feature = "gzip")]
use std::io::Write;
use std::net::SocketAddr;

use SysinfoExt;
use serde_json;

use warp::{self, Filter, Rejection, Reply};
use warp::http::{Response, StatusCode};

const INDEX_HTML: &'static [u8] = include_bytes!("index.html");
const FAVICON: &'static [u8] = include_bytes!("../resources/favicon.ico");
const REFRESH_DELAY: u64 = 60 * 10; // 10 minutes

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
    ($encoding:expr, $content:expr, $typ:expr) => {{
         let s = $encoding.to_lowercase();
         if s.contains("gzip") || s.contains("*") {
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
    }}
}

#[cfg(not(feature = "gzip"))]
macro_rules! return_gzip_or_not {
    ($encoding:expr, $content:expr, $typ:expr) => {{
        Response::builder()
                 .header("content-type", $typ)
                 .body($content.to_owned())
    }}
}

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

    let index = warp::path("favicon.ico")
                     .and(warp::header::<String>("accept-encoding"))
                     .map(|_encoding: String| {
        return_gzip_or_not!(_encoding, FAVICON, "image/x-icon")
    });
    let update = warp::path("sysinfo.json")
                     .and(warp::header::<String>("accept-encoding"))
                     .map(move |_encoding: String| {
        data_handler.update_last_connection();
        let x = data_handler.json_output.read().unwrap().clone().as_bytes().to_owned();
        return_gzip_or_not!(_encoding,
                            x.as_slice(),
                            "application/json")
    });
    let index2 = warp::index()
                     .and(warp::header::<String>("accept-encoding"))
                     .map(|_encoding: String| {
        return_gzip_or_not!(_encoding, INDEX_HTML, "text/html")
    });

    let routes = warp::get2().and(index.or(update).or(index2)).recover(customize_error);

    match sock_addr {
        Some(s) => {
            let addr: SocketAddr = match s.parse() {
                Ok(a) => a,
                Err(e) => {
                    println!("Invalid IP address: {:?}", e);
                    return Err(());
                }
            };
            warp::serve(routes).run(addr);
        }
        None => {
            println!("Starting on 127.0.0.1:4321");
            warp::serve(routes).run(([127, 0, 0, 1], 4321));
        }
    }
    Ok(())
}
