use axum::{Router, Server};

use crate::config::Config;

#[tracing::instrument]
pub async fn run(cfg: Config) {
    // let rt = Router::new();
    for sock in cfg.listen.sockets {
        // Server::bind().serve().await.unwrap();
    }
}
