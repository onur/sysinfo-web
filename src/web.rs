
use iron::{Iron, IronResult, Listening, status};
use iron::error::HttpResult;
use iron::response::Response;
use iron::request::Request;
use iron::middleware::Handler;
use iron::mime::Mime;

use sysinfo::{System, SystemExt};
use std::sync::{Arc, Mutex};
use SysinfoExt;
use serde_json;


const INDEX_HTML: &'static str = include_str!("index.html");

struct SysinfoIronHandler(Arc<Mutex<System>>);


impl Handler for SysinfoIronHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.url.path().last() {
            Some(path) => {
                if *path == "" {
                    Ok(Response::with((status::Ok,
                                       "text/html".parse::<Mime>().unwrap(),
                                       INDEX_HTML)))
                } else {
                    let mut system = self.0.lock().unwrap();
                    system.refresh_all();
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
    let system = Arc::new(Mutex::new(System::new()));
    let mut iron = Iron::new(SysinfoIronHandler(system));
    iron.threads = 4;
    iron.http(sock_addr.unwrap_or("localhost:3000".to_owned()))
}


#[test]
fn test_sysinfo_json_str() {
    assert!(!sysinfo_json_str().is_empty());
}
