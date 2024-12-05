use crate::cli::CtlCommand;
use bytes::{Buf, Bytes};
use http_body_util::{BodyExt, Empty};
use hyper::{
    client::conn::http1::{Connection, SendRequest},
    rt::Executor,
    Request,
};
use std::path::PathBuf;
use tokio::net::UnixStream;

pub async fn run(
    cfg: crate::config::Config,
    socket: PathBuf,
    cmd: CtlCommand,
) -> Result<(), std::io::Error> {
    let stream = UnixStream::connect(&socket).await?;
    match cmd {
        CtlCommand::PrintCfg => {
            todo!();
            //let rem_cfg = get_cfg(cfg, socket, stream).await.unwrap();
            //println!("{}", toml::to_string_pretty(&rem_cfg).unwrap());
        }
        cmd => todo!("{cmd:?}"),
    }
    Ok(())
}

//pub async fn get_cfg(
//    cfg: crate::config::Config,
//    socket: PathBuf,
//    stream: UnixStream,
//) -> hyper::Result<crate::config::Config> {
//    let (mut sender, conn): (SendRequest<_>, Connection<_, _>) =
//        hyper::client::conn::http1::handshake(/* TokioExecutor, */ stream).await?;
//    tokio::task::spawn(async move {
//        if let Err(e) = conn.await {
//            tracing::error!(error = ?e, "connection failed")
//        }
//    });
//
//    let response = sender
//        .send_request(
//            Request::builder()
//                .uri("/ctl?config")
//                .header(hyper::header::HOST, socket.as_os_str().to_str().unwrap())
//                .method("GET")
//                .body(Empty::<Bytes>::new())
//                .unwrap(),
//        )
//        .await?;
//
//    let body = response.collect().await?.aggregate();
//
//    Ok(serde_json::from_reader::<_, crate::config::Config>(body.reader()).unwrap())
//}
