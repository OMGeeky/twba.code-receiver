use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::fs::write;
use tracing::instrument;
use twba_backup_config::Conf;
use twba_common::prelude::*;
use url::Url;

#[tokio::main]
#[instrument]
async fn main() {
    let _guard = init_tracing("twba_code_receiver");
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let addr = ([0, 0, 0, 0], 3000).into();

    let server = Server::bind(&addr).serve(make_svc);

    info!("Starting code receiver");
    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
}
#[instrument]
async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/googleapi/auth") => auth_get(req).await,
        (&Method::GET, "/") => Ok(Response::new(Body::from("Hello, World!"))),
        (&Method::GET, "/favicon.ico") => Ok(Response::default()),
        other => {
            error!("404: {:?} {:?}", other.0, other.1);
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn auth_get(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let url = format!("http://localhost{}", req.uri());
    trace!("auth get request with url: '{}'", url);
    let url = Url::parse(&url).unwrap();
    let params: HashMap<_, _> = url.query_pairs().collect();
    if let Some(code) = params.get("code") {
        info!("Code received: '{}'", code);
        let write_res = write_to_file(code.to_string()).await;
        match write_res {
            Ok(_) => {
                info!("Code written to file");
                Ok(Response::new(Body::from("Code written to file")))
            }
            Err(e) => {
                error!("Error writing code to file: {e:?}");
                Ok(Response::new(Body::from(
                    "Error writing code to file: {e:?}",
                )))
            }
        }
    } else {
        error!("No code provided");
        Ok(Response::new(Body::from("No code provided")))
    }
}

lazy_static! {
    static ref CONF: Conf = twba_backup_config::get_default_builder()
        .load()
        .expect("Failed to load config");
    static ref AUTH_CODE_PATH: String = CONF.google.path_auth_code.clone();
}
async fn write_to_file(code: String) -> std::io::Result<()> {
    let path = AUTH_CODE_PATH.to_string();
    println!("writing code '{}' to file: '{}'", code, path);
    trace!("writing code '{}' to file: '{}'", code, path);
    write(path, code).await
}
