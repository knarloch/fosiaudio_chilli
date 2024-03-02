mod player;
mod volume_controler;

use clap::Parser;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc};
use tokio::net::TcpListener;

use player::Player;
use volume_controler::VolumeControler;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = ("0.0.0.0:80".to_string()))]
    socket_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = Args::parse().socket_addr.parse()?;

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    let player = Arc::new(Player::new());
    let volume_controler = Arc::new(VolumeControler::new());

    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let player = player.clone();
        let volume_controler = volume_controler.clone();

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(
                    io,
                    service_fn(move |request| {
                        hello(request, player.clone(), volume_controler.clone())
                    }),
                )
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

fn report_internal_server_error(error: Box<dyn std::error::Error>) -> Response<Full<Bytes>> {
    let mut server_error = Response::new(Full::new(error.to_string().into()));
    *server_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    server_error
}

async fn hello(
    request: Request<hyper::body::Incoming>,
    player: Arc<Player>,
    volume_controler: Arc<VolumeControler>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let html: &str = r#"<!DOCTYPE html>
        <html lang="en">
        <head>
        <meta charset="UTF-8">
        <title>fosiaudio_chilli</title>
        </head>
        <body>
        <span style="font-size:8em;"><p><a href="/play">play</a></p></span>
        <span style="font-size:8em;"><p><a href="/pause">pause</a></p></span>
        <span style="font-size:4em;"><p><a href="/change_volume?10">louder!</a></p></span>
        <span style="font-size:4em;"><p><a href="/change_volume?1">louder</a></p></span>
        <span style="font-size:4em;"><p><a href="/change_volume?-1">softer</a></p></span>
        <span style="font-size:4em;"><p><a href="/change_volume?-10">softer!</a></p></span>
        </body>
        </html>"#;

    let uri = request.uri().path();
    println!("Requested uri: \"{}\"", uri);
    let uri = request.uri().path();
    match uri {
        "/" => Ok(Response::new(Full::new(Bytes::from(html)))),
        "/play" => match player.play() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(err) => Ok(report_internal_server_error(err.into())),
        },
        "/pause" => match player.pause() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(err) => Ok(report_internal_server_error(err.into())),
        },
        "/change_volume" => {
            let param = request.uri().query().unwrap_or("").parse::<i32>();
            match param {
                Ok(vol_delta) =>
                    match volume_controler.change_volume(vol_delta) {
                        Ok(_) => Ok(Response::new(Full::new(Bytes::from(html)))),
                        Err(err) => Ok(report_internal_server_error(err)),
                },
                    Err(err) => Ok(report_internal_server_error(err.into()))
                }
        },
        _ => {
            let mut not_found = Response::new(Full::new(Bytes::new()));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
