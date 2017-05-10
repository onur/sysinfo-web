
use iron::{Iron, IronResult, Listening, status};
use iron::error::HttpResult;
use iron::response::Response;
use iron::request::Request;
use iron::middleware::Handler;
use iron::mime::Mime;

use sysinfo::{System, SystemExt};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use SysinfoExt;
use serde_json;


const INDEX_HTML: &'static str = include_str!("index.html");
const REFRESH_DELAY: u64 = 60 * 10; // 10 minutes

struct SysinfoIronHandler(Arc<DataHandler>);

struct DataHandler {
    system: RwLock<System>,
    last_connection: Mutex<SystemTime>,
}

impl DataHandler {
    fn can_update_system_info(&self) -> bool {
        let old = {
            if let Ok(s) = self.last_connection.lock() {
                (*s).clone()
            } else {
                SystemTime::now()
            }
        };
        SystemTime::now().duration_since(old)
                         .unwrap()
                         .as_secs() < REFRESH_DELAY
    }

    fn update_last_connection(&self) {
        if let Ok(mut s) = self.last_connection.lock() {
            *s = SystemTime::now();
        }
    }
}

impl Handler for SysinfoIronHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.url.path().last() {
            Some(path) => {
                if *path == "" {
                    Ok(Response::with((status::Ok,
                                       "text/html".parse::<Mime>().unwrap(),
                                       INDEX_HTML)))
                } else {
                    self.0.update_last_connection();
                    let system = self.0.system.read().unwrap();
                    let sysinfo = SysinfoExt::new(&system);
                    Ok(Response::with((status::Ok,
                                       "application/json".parse::<Mime>().unwrap(),
                                       serde_json::to_string(&sysinfo).unwrap_or(String::new()))))
                }
            },
            None => Ok(Response::with((status::NotFound, "Not found")))
        }
    }
}


pub fn start_web_server(sock_addr: Option<String>) -> HttpResult<Listening> {
    let data_handler = Arc::new(DataHandler {
        system: RwLock::new(System::new()),
        last_connection: Mutex::new(SystemTime::now()),
    });
    let data_handler_clone = data_handler.clone();
    thread::spawn(move || {
        loop {
            if data_handler_clone.can_update_system_info() {
                {
                    let mut system = data_handler_clone.system.write().unwrap();
                    system.refresh_all();
                }
                thread::sleep(Duration::new(5, 0));
            } else {
                // If we don't need to refresh the system information, we can sleep a lot less.
                thread::sleep(Duration::from_millis(500));
            }
        }
    });
    let mut iron = Iron::new(SysinfoIronHandler(data_handler));
    iron.threads = 4;
    iron.http(sock_addr.unwrap_or("localhost:3000".to_owned()))
}
