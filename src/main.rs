#[macro_use] extern crate protocol_derive;
#[macro_use] extern crate protocol;

extern crate clap;
extern crate bus;
extern crate hyper;
extern crate futures;

use clap::{Arg, App};
use hyper::server::Http;

mod remote_play;
mod server;

use server::StreamingServer;
use remote_play::stream;

fn main() {
    let matches = App::new("ntr_stream")
        .arg(Arg::with_name("host")
            .index(1)
            .required(true)
            .help("hostname or ip address of 3ds to connect to")
            .takes_value(true))
        .arg(Arg::with_name("quality")
            .short("q")
            .long("quality")
            .help("JPEG compression quality. Must be an integer between 10 and 100")
            .takes_value(true)
            .default_value("90"))
        .arg(Arg::with_name("port")
            .short("p")
            .long("listen-port")
            .help("Port for local http server to listen on")
            .takes_value(true)
            .default_value("9090"))
        .get_matches();


    let quality: u32 = matches.value_of("quality").unwrap().parse()
        .expect("Quality must be a valid integer value");
    let host = matches.value_of("host").unwrap();
    let listen_port = matches.value_of("port").unwrap();

    println!("Host: {}, quality: {}", host, quality);
    println!("port: {}", listen_port);


    let addr = format!("127.0.0.1:{}", listen_port);
    let addr = addr.parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(
        StreamingServer { image_readers: stream::remote_play_stream() }
    )).unwrap();
    server.run().unwrap();
}
