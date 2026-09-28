use color_eyre::eyre::Result;
use http_body_util::Full;
use hyper::{
    Request, Response, StatusCode, Uri,
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use std::{
    env::current_dir,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tokio::net::TcpListener;
use tracing::{Level, debug, info, warn};

static CWD: OnceLock<PathBuf> = OnceLock::new();

fn get_cwd() -> &'static PathBuf {
    CWD.get_or_init(|| match current_dir() {
        Ok(p) => p,
        _ => unimplemented!(),
    })
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    debug!("{}", req.uri());

    let uri = req.uri();
    let cwd = get_cwd();

    // Get the file path, if it doesn't exist return 404.
    let path = match Path::new(&format!(".{}", uri.path())).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("404 Not Found")))?);
        }
    };

    if path.is_dir() {
        return construct_dir_view(&path, &uri);
    }

    Ok(Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .body(Full::new(Bytes::from("501 Not Implemented")))?)
}

fn construct_dir_view(path: &PathBuf, uri: &Uri) -> Result<Response<Full<Bytes>>> {
    let mut body = String::new();

    body.push_str(&format!("<h1>Index of {}</h1>", uri.path()));

    let entries = path.read_dir()?;

    let mut dir_entries = Vec::new();
    let mut file_entries = Vec::new();

    for entry in entries.flatten() {
        let ep = entry.path(); // entry_path
        let ep_str = ep.to_string_lossy().into_owned();

        if ep.is_dir() {
            dir_entries.push(ep_str);
        } else if ep.is_file() {
            file_entries.push(ep_str);
        }
    }

    dir_entries.sort();
    file_entries.sort();

    let mut entries = dir_entries;
    entries.append(&mut file_entries);

    if uri != "/" {
        body.push_str(&format!("<a href=\"{}..\"><p>..</p></a>", uri));
    }

    for e in entries {
        body.push_str(&format!(
            "<a href=\"{}/{}/\"><p>{}{}</p></a>",
            uri.path().trim_end_matches("/"),
            PathBuf::from(e.clone()).strip_prefix(path)?.display(),
            PathBuf::from(e.clone()).strip_prefix(path)?.display(),
            if PathBuf::from(e).is_dir() { "/" } else { "" }
        ));
    }

    Ok(Response::new(Full::new(Bytes::from(body))))

    // Ok(Response::builder()
    //     .status(StatusCode::NOT_IMPLEMENTED)
    //     .body(Full::new(Bytes::from("501 Not Implemented")))?)
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
