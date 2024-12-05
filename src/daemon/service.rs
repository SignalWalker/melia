use bytes::Bytes;
use crossbeam::sync::ShardedLock;
use futures::FutureExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body;
use hyper::{Method, Request, Response, StatusCode};
use std::pin::Pin;
use std::sync::Arc;

// type PinFuture<Output> = Pin<Box<dyn Future<Output = Output> + Send>>;

pub type Result<O> = std::result::Result<O, ServiceError>;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Http(#[from] hyper::http::Error),
    #[error("infallible...?")]
    Infallible,
}

impl From<std::convert::Infallible> for ServiceError {
    fn from(_: std::convert::Infallible) -> Self {
        Self::Infallible
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ServiceConfig {
    tls: bool,
    allow_ctl: bool,
}

impl ServiceConfig {
    pub const UNIX: Self = Self {
        tls: false,
        allow_ctl: true,
    };
    pub const HTTP: Self = Self {
        tls: false,
        allow_ctl: false,
    };
    pub const HTTPS: Self = Self {
        tls: true,
        allow_ctl: false,
    };
}

#[derive(Debug, Clone)]
pub struct Service {
    pub cfg: Arc<ShardedLock<crate::config::Config>>,
    pub allow_ctl: bool,
}

impl Service {
    pub fn new(cfg: Arc<ShardedLock<crate::config::Config>>, svc_cfg: &ServiceConfig) -> Self {
        Self {
            cfg,
            allow_ctl: svc_cfg.allow_ctl,
        }
    }
}

impl hyper::service::Service<Request<body::Incoming>> for Service {
    type Response = Response<BoxBody<Bytes, ServiceError>>;
    type Error = ServiceError;
    type Future = Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Self::Response, Self::Error>>
                + Send,
        >,
    >;

    fn call(&self, req: Request<body::Incoming>) -> Self::Future {
        tracing::debug!(request = ?req, "received request");
        respond(self.cfg.clone(), self.allow_ctl, req).boxed()
    }
}

// type SvcResponse = <Svc as Service<Request<body::Incoming>>>::Response;
// type SvcError = <Svc as Service<Request<body::Incoming>>>::Error;

pub async fn respond(
    cfg: Arc<ShardedLock<crate::config::Config>>,
    allow_ctl: bool,
    req: Request<body::Incoming>,
) -> Result<Response<BoxBody<Bytes, ServiceError>>> {
    fn mk_response(s: impl ToString) -> Result<Response<BoxBody<Bytes, ServiceError>>> {
        Response::builder()
            .body(full(Bytes::from(s.to_string())))
            .map_err(ServiceError::from)
    }
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => mk_response(
            "Try POSTing data to /echo (ex. `curl localhost:8080/echo -XPOST -d \"Hello, World\"`)",
        ),
        (&Method::POST, "/echo") => Ok(Response::new(
            req.into_body().map_err(ServiceError::from).boxed(),
        )),
        (_, "/api") if allow_ctl => respond_api(&cfg, req).await,
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, ServiceError> {
    Full::new(chunk.into()).map_err(|n| match n {}).boxed()
}

async fn respond_api(
    cfg: &Arc<ShardedLock<crate::config::Config>>,
    req: Request<body::Incoming>,
) -> Result<Response<BoxBody<Bytes, ServiceError>>> {
    match (req.method(), req.uri().query()) {
        (&Method::GET, Some("config")) => Ok(Response::builder()
            .body(
                serde_json::to_string(&*cfg.read().unwrap())
                    .unwrap()
                    .map_err(ServiceError::from)
                    .boxed(),
            )
            .unwrap()),
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
