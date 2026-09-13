use color_eyre::eyre::Result;
use http_body_util::Full;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{Level, info, warn};

async fn handle_request(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    Ok(Response::new(Full::new(Bytes::from("Hello, world!"))))
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(address).await?;

    info!("started HTTP/1 server on port 3000");

    loop {
        let (stream, _addr) = listener.accept().await?;

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            match http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                Ok(_) => {}
                Err(e) => warn!("error serving connection: {}", e),
            }
        });
    }
}
