use futures::{future, Sink};
use futures::future::Future;
use futures::sync::oneshot;
use futures::sync::mpsc;
use std::thread;

use hyper;
use hyper::server::{Request, Response, Service};
use hyper::Method;
use hyper::StatusCode;

use bus::BusReader;
use remote_play::stream::RemotePlayStream;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use hyper::Chunk;

pub struct StreamingServer {
    pub remote_play_stream: RemotePlayStream
}

fn format_timestamp() -> String {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let in_ms = ts.as_secs() * 1000 + ts.subsec_nanos() as u64 / 1_000_000;
    format!("{:6}", in_ms as f64)
}


fn stream_images(mut rx_frames: BusReader<Vec<u8>>) -> Box<Future<Item=hyper::Response, Error=hyper::Error>>{
    let mut response = Response::new();
    let (tx_response, rx_response) = oneshot::channel();
    let (mut tx_body, rx_body) = mpsc::channel(1);

    thread::spawn(move || {
        // TODO: error handling for response?
        tx_response.send(response.with_body(rx_body)).expect("Error sending response");

        let boundary = [
            "contentboundary".to_string(),
            "Content-Type: image/jpeg".to_string(),
            format!("X-StartTime: {}", format_timestamp())
        ].join("\r\n");

        loop {
            let frame_result = rx_frames.recv();
            if frame_result.is_err() {
                continue;
            }
            let frame = frame_result.unwrap();
            let timestamp_header = format!("\r\nX-Timestamp: {}\r\n\r\n", format_timestamp());
            let mut chunk: Chunk = boundary.clone().into_bytes().into();
            chunk.extend(timestamp_header.into_bytes());
            chunk.extend(frame);
            match tx_body.send(Ok(chunk)).wait() {
                Ok(t) => { tx_body = t; },
                Err(_) => { break; }
            }
        }
    });

    Box::new(rx_response.map_err(|e| hyper::Error::from(io::Error::new(io::ErrorKind::Other, e))))
}


impl Service for StreamingServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut response = Response::new();

        match (req.method(), req.path()) {
            (&Method::Get, "/") => {
                response.set_body("hi there");
            },

            (&Method::Get, "/top") => {
                let bus = self.remote_play_stream.top_image_bus.clone();
                let mut rx = bus.lock().unwrap().add_rx();
                return stream_images(rx);
            }

            _ => {
                response.set_status(StatusCode::NotFound);
            },
        };

        Box::new(future::ok(response))
    }
}

