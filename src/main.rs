use std::convert::Infallible;
use std::io::Error;
use std::net::SocketAddr;
use std::process::{Child, Command};

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::{Arc, Mutex};
use regex::Regex;
use tokio::net::TcpListener;
use clap::Parser;

struct Player {
    player_instance: Mutex<Option<Child>>,
}

impl Player {
    fn new() -> Player {
        Player {
            player_instance: Mutex::new(None),
        }
    }
}

impl Player {
    fn play(&self) -> Result<(), std::io::Error> {
        let player = &mut *self.player_instance.lock().unwrap();
        match player {
            Some(_) => Ok(()),
            None => {
                let spawn_result = Command::new("cvlc")
                    .arg("https://n-22-14.dcs.redcdn.pl/sc/o2/Eurozet/live/chillizet.livx")
                    .spawn()?;
                *player = Some(spawn_result);
                Ok(())
            }
        }
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

fn get_current_volume() -> Result<i32, Box<dyn std::error::Error >> {
    let output = String::from_utf8(
        Command::new("amixer")
            .args(["-c", "2", "sget", "'PCM',0"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let re = Regex::new(r"\[(?<percent>\d+)%\]").unwrap();
    let caps = re.captures(&*output).unwrap();
    let percent = &caps["percent"];
    let result :i32 = percent.parse()?;

    println!("Current volume: {}", result);
    return Ok(result);
}

fn set_current_volume(vol : i32) -> Result<(), std::io::Error> {
    let vol_percent  = vol.to_string() + "%";

        Command::new("amixer")
            .args(["-c", "2", "sset", "'PCM',0", &*vol_percent]).status()
        .unwrap();
    Ok(())
}

fn change_volume(command : &str) ->Result<(), std::io::Error> {
    let vol_diff : i32 =command.parse().unwrap_or(0).clamp(-100, 100);
    let vol = get_current_volume().unwrap() + vol_diff;
    set_current_volume(vol.clamp(0, 100))
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = ("0.0.0.0:80".to_string()))]
    socket_addr: String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr : SocketAddr = Args::parse().socket_addr.parse()?;

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

fn report_internal_server_error(error: Error) -> Response<Full<Bytes>> {
    let mut server_error = Response::new(Full::new(error.to_string().into()));
    *server_error.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    server_error
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
        <h3><p><a href="/volume?+10">louder!</a></p></h3>
        <h3><p><a href="/volume?+1">louder</a></p></h3>
        <h3><p><a href="/volume?-1">softer</a></p></h3>
        <h3><p><a href="/volume?-10">softer!</a></p></h3>
        </body>
        </html>"#;

    let uri = request.uri().path();
    println!("Requested uri: \"{}\"", uri);
    let uri = request.uri().path();
    match uri {
        "/" => Ok(Response::new(Full::new(Bytes::from(html)))),
        "/play" => match player.play() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(err) => Ok(report_internal_server_error(err)),
        },
        "/pause" => match player.pause() {
            Ok(()) => std::prelude::rust_2015::Ok(Response::new(Full::new(Bytes::from(html)))),
            Err(err) => Ok(report_internal_server_error(err)),
        },
        "/volume" => {
            match change_volume(request.uri().query().unwrap()) {
                Ok(()) => Ok(Response::new(Full::new(Bytes::from(html)))),
                Err(err) => Ok(report_internal_server_error(err)),
            }
        },
        _ => {
            let mut not_found = Response::new(Full::new(Bytes::new()));
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
