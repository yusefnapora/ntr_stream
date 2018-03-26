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
use remote_play::stream::ImageReaders;
use std::io;
use hyper::Chunk;
use futures::sync::mpsc::Receiver;

pub struct StreamingServer {
    pub image_readers: ImageReaders
}

fn format_timestamp() -> String {
    "foo".to_string()
}


fn stream_images(rx_frames: Receiver<Vec<u8>>) -> Box<Future<Item=hyper::Response, Error=hyper::Error>>{
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
            let frame = rx_frames.recv();
            let timestamp_header = format!("\r\nX-Timestamp: {}\r\n\r\n", format_timestamp());
            let mut chunk: Chunk = boundary.into_bytes().into();
            chunk.extend(timestamp_header.into_bytes());
            chunk.extend(frame);
//            tx_body.send(Ok(chunk)).wait().expect("Error sending frame");
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

//            (&Method::Get, "/top") => {
//                return stream_images(top_reader.clone(), response);
//            }

            _ => {
                response.set_status(StatusCode::NotFound);
            },
        };

        Box::new(future::ok(response))
    }
}

