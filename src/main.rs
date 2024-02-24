use std::convert::Infallible;
use std::net::SocketAddr;
use std::process::{Child, Command};

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

struct Player {
    player_instance: Mutex<Option<Child>>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            player_instance: Mutex::new(None),
        }
    }
}

impl Player {
    fn play(&self) -> Result<(), std::io::Error> {
        let player = &mut *self.player_instance.lock().unwrap();
        if player.is_none() {
            let spawn_result = Command::new("cvlc")
                .arg("https://n-22-14.dcs.redcdn.pl/sc/o2/Eurozet/live/chillizet.livx")
                .spawn()?;
            *player = Some(spawn_result);
            return Ok(());
        }
        Ok(())
    }
}

impl Player {
    fn pause(&self) -> Result<(), std::io::Error> {
        let player = &mut *self.player_instance.lock().unwrap();
        match player {
            Some(process) => {
                process.kill()?;
                *player = None;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    let player = Arc::new(Player::new());

    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let player = player.clone();

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(
                    io,
                    service_fn(move |request| hello(request, player.clone())),
                )
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn hello(
    request: Request<hyper::body::Incoming>,
    player: Arc<Player>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let html: &str = r#"<!DOCTYPE html>
        <html lang="en">
        <head>
        <meta charset="UTF-8">
        <title>fosiaudio_chilli</title>
        </head>
        <body>
        <h1><p><a href="/play">play</a></p></h1>
        <h1><p><a href="/pause">pause</a></p></h1>
        </body>
        </html>"#;

    let uri = request.uri().path();
    println!("Requested uri: \"{}\"", uri);
    match request.uri().path() {
        "/" => Ok(Response::new(Full::new(Bytes::from(html)))),
        "/play" => match player.play() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(E) => {
                let mut server_error = Response::new(Full::new(E.to_string().into()));
                *server_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                Ok(server_error)
            }
        },
        "/pause" => match player.pause() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(E) => {
                let mut server_error = Response::new(Full::new(E.to_string().into()));
                *server_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                Ok(server_error)
            }
        },
        _ => {
            let mut not_found = Response::new(Full::new(Bytes::new()));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
