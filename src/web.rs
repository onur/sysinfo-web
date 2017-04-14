
use iron::{Iron, IronResult, Listening, status};
use iron::error::HttpResult;
use iron::response::Response;
use iron::request::Request;
use iron::middleware::Handler;
use iron::mime::Mime;

use sysinfo::{System, SystemExt};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use SysinfoExt;
use serde_json;


const INDEX_HTML: &'static str = include_str!("index.html");

struct SysinfoIronHandler(Arc<RwLock<System>>);


impl Handler for SysinfoIronHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.url.path().last() {
            Some(path) => {
                if *path == "" {
                    Ok(Response::with((status::Ok,
                                       "text/html".parse::<Mime>().unwrap(),
                                       INDEX_HTML)))
                } else {
                    let system = self.0.read().unwrap();
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
    let system = Arc::new(RwLock::new(System::new()));
    let system_clone = system.clone();
    thread::spawn(move || {
        loop {
            {
                let mut system = system_clone.write().unwrap();
                system.refresh_all();
            }
            thread::sleep(Duration::new(5, 0));
        }
    });
    let mut iron = Iron::new(SysinfoIronHandler(system));
    iron.threads = 4;
    iron.http(sock_addr.unwrap_or("localhost:3000".to_owned()))
}
