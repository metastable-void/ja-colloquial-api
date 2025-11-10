
use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use hyper::http::StatusCode;

fn make_json_response(code: hyper::http::StatusCode, json: serde_json::Value) -> Response<Full<Bytes>> {
    let value = serde_json::to_string(&json).unwrap();
    let mut res = Response::new(Full::new(Bytes::from(value)));
    *(res.status_mut()) = code;
    res.headers_mut().insert("content-type", "application/json".try_into().unwrap());
    res.headers_mut().insert("access-control-allow-origin", "*".try_into().unwrap());
    res
}

async fn api(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let uri = req.uri().to_string();
    let parts = uri.split("?").collect::<Vec<_>>();
    let parts = parts[0].split("/").collect::<Vec<_>>();
    if !parts[0].is_empty() || parts.len() < 2 {
        return Ok(make_json_response(StatusCode::BAD_REQUEST, serde_json::json!({
            "error": "Invalid request",
        })));
    }

    let books = ja_colloquial::books();
    match parts[1] {
        "" => {
            let verse = books.random_verse();
            Ok(make_json_response(StatusCode::OK, serde_json::json!({
                "error": serde_json::Value::Null,
                "verse": serde_json::json!({
                    "book": verse.jb,
                    "chapter": verse.c,
                    "verse": verse.v,
                    "text": verse.t,
                })
            })))
        },

        b => {
            if parts.len() != 4 {
                Ok(make_json_response(StatusCode::BAD_REQUEST, serde_json::json!({
                    "error": "Invalid request",
                })))
            } else {
                let chapter = parts[2].parse::<u8>().unwrap_or(0);
                let verse = parts[3].parse::<u8>().unwrap_or(0);

                let verse = books.get_verse(b, chapter, verse);
                if let Some(verse) = verse {
                    Ok(make_json_response(StatusCode::OK, serde_json::json!({
                        "error": serde_json::Value::Null,
                        "verse": serde_json::json!({
                            "book": verse.jb,
                            "chapter": verse.c,
                            "verse": verse.v,
                            "text": verse.t,
                        })
                    })))
                } else {
                    Ok(make_json_response(StatusCode::NOT_FOUND, serde_json::json!({
                        "error": "Not found",
                    })))
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = std::env::var("LISTEN_ADDR").ok()
        .map(|a| a.parse::<SocketAddr>().ok()).flatten()
        .unwrap_or(SocketAddr::from(([127, 0, 0, 1], 3333)));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service_fn(api))
                .await
            {
                eprintln!("Error serving connection: {}", e);
            }
        });
    }
}
