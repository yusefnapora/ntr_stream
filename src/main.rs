#[macro_use] extern crate protocol_derive;
#[macro_use] extern crate protocol;

extern crate clap;
extern crate bus;
extern crate hyper;
extern crate futures;
extern crate byteorder;

use clap::{Arg, App};
use hyper::server::Http;

mod remote_play;
mod server;

use server::StreamingServer;
use remote_play::packets::StreamingConfig;
use remote_play::stream;
use remote_play::packets::Screen;

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

    let config = StreamingConfig {
        host: host.to_string(),
        priority_factor: 1,
        priority_screen: Screen::Top,
        compression_quality: quality,
        qos: 900
    };

    println!("Host: {}, quality: {}", host, quality);
    println!("port: {}", listen_port);

    stream::send_init_command(&config);

    let addr = format!("127.0.0.1:{}", listen_port);
    let addr = addr.parse().unwrap();

    let server = Http::new().bind(&addr, || Ok(
        StreamingServer { remote_play_stream: stream::RemotePlayStream::bind().expect("error starting remote play stream") }
    )).unwrap();
    server.run().unwrap();

}
