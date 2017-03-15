extern crate sysinfo_web;

fn main() {
    sysinfo_web::start_web_server(::std::env::args().nth(1)).unwrap();
}
