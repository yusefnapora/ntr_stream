use futures::future;
use futures::future::Future;

use hyper;
use hyper::server::{Request, Response, Service};
use hyper::Method;
use hyper::StatusCode;

pub struct StreamingServer;

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

            _ => {
                response.set_status(StatusCode::NotFound);
            },
        };

        Box::new(future::ok(response))
    }
}

