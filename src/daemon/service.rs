use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body;
use hyper::{service::Service, Method, Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

type PinFuture<Output> = Pin<Box<dyn Future<Output = Output> + Send>>;

pub struct Svc;
impl Service<Request<body::Incoming>> for Svc {
    type Response = Response<BoxBody<Bytes, hyper::Error>>;
    type Error = hyper::Error;
    type Future = futures_util::future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&mut self, req: Request<body::Incoming>) -> Self::Future {
        tracing::info!(request = ?req, "received request");
        todo!()
        // futures_util::future::ok(respond(req))
    }
}

// type SvcResponse = <Svc as Service<Request<body::Incoming>>>::Response;
// type SvcError = <Svc as Service<Request<body::Incoming>>>::Error;

pub async fn respond(
    req: Request<body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    fn mk_response(s: String) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        Ok(Response::builder().body(full(Bytes::from(s))).unwrap())
    }
    tracing::debug!(request = ?req, "responding to request");
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => mk_response(
            "Try POSTing data to /echo (ex. `curl localhost:8080/echo -XPOST -d \"Hello, World\"`)"
                .into(),
        ),
        (&Method::POST, "/echo") => Ok(Response::new(req.into_body().boxed())),
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into()).map_err(|n| match n {}).boxed()
}
