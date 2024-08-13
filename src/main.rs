mod player;
mod volume_controler;

use clap::Parser;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, Empty};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use player::Player;
use volume_controler::VolumeControler;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:80")]
    socket_addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = Args::parse().socket_addr.parse()?;

    let listener = TcpListener::bind(addr).await?;

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

fn report_internal_server_error(
    error: Box<dyn std::error::Error>,
) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::new(error.to_string().into()).boxed())
        .unwrap()
}

fn redirect_to_root() -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(http::header::LOCATION, "/")
        .body(Empty::<Bytes>::new().boxed())
        .unwrap()
        .into()
}

fn respond_with_root() -> Response<BoxBody<Bytes, Infallible>> {
    let html: &'static str = include_str!("fosiaudio_chilli.html");
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from_static(html.as_bytes())).boxed())
        .unwrap()
}

fn respond_not_found() -> Response<BoxBody<Bytes, Infallible>> {

    let mut response = Response::new(Empty::<Bytes>::new().boxed());
    *response.status_mut() = StatusCode::NOT_FOUND;
    return response.into();
}

async fn hello(
    request: Request<hyper::body::Incoming>,
    player: Arc<Player>,
    volume_controler: Arc<VolumeControler>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let uri = request.uri().path();
    println!("Requested uri: \"{}\"", uri);
    let uri = request.uri().path();
    match uri {
        "/" => Ok(respond_with_root()),
        "/play" => match player.play("https://r.dcs.redcdn.pl/sc/o2/Eurozet/live/chillizet.livx".into()) {
            Ok(()) => Ok(redirect_to_root()),
            Err(err) => Ok(report_internal_server_error(err.into())),
        },
        "/pause" => match player.pause() {
            Ok(()) => Ok(redirect_to_root()),
            Err(err) => Ok(report_internal_server_error(err.into())),
        },
        "/change_volume" => {
            let param = request.uri().query().unwrap_or("").parse::<i32>();
            match param {
                Ok(vol_delta) => match volume_controler.change_volume(vol_delta) {
                    Ok(_) => Ok(redirect_to_root()),
                    Err(err) => Ok(report_internal_server_error(err)),
                },
                Err(err) => Ok(report_internal_server_error(err.into())),
            }
        }
        _ => Ok(respond_not_found()),
    }
}
