
use iron::{Iron, IronResult, Listening, status};
use iron::error::HttpResult;
use iron::response::Response;
use iron::request::Request;
use iron::middleware::Handler;
use iron::mime::Mime;


const INDEX_HTML: &'static str = include_str!("index.html");

struct SysinfoIronHandler;


impl Handler for SysinfoIronHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        match req.url.path().last() {
            Some(path) => {
                if *path == "" {
                    Ok(Response::with((status::Ok,
                                       "text/html".parse::<Mime>().unwrap(),
                                       INDEX_HTML)))
                } else {
                    Ok(Response::with((status::Ok,
                                       "application/json".parse::<Mime>().unwrap(),
                                       sysinfo_json_str())))
                }
            },
            None => Ok(Response::with((status::NotFound, "Not found")))
        }
    }
}


pub fn start_web_server(sock_addr: Option<String>) -> HttpResult<Listening> {
    let mut iron = Iron::new(SysinfoIronHandler);
    iron.threads = 2;
    iron.http(sock_addr.unwrap_or("localhost:3000".to_owned()))
}


fn sysinfo_json_str() -> String {
    use SysinfoExt;
    use sysinfo::{System, SystemExt};
    use serde_json;
    let mut system = System::new();
    system.refresh_all();
    let sysinfo = SysinfoExt::new(&system);
    serde_json::to_string(&sysinfo).unwrap_or(String::new())
}


#[test]
fn test_sysinfo_json_str() {
    assert!(!sysinfo_json_str().is_empty());
}
