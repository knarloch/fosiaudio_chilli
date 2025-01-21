mod autogrzybke;
mod http_request_handler;
mod player;
mod schedule;
mod volume_controller;

use crate::autogrzybke::Autogrzybke;
use crate::schedule::Scheduler;
use crate::volume_controller::VolumeController;
use anyhow::Context;
use clap::Parser;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::*;
use player::Player;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0:80")]
    socket_addr: String,
    #[arg(short, long, default_value = "/opt/autogrzybke_resources")]
    autogrzybke_resources_path: String,
    #[arg(short, long, default_value = "ffplay")]
    ffplay_path: String,
    #[arg(long, default_value = "33")]
    prefix_chance_percent: u64,
    #[arg(long, default_value = "33")]
    suffix_chance_percent: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::Level::Info)
        .init()
        .unwrap();

    let addr: SocketAddr = Args::parse().socket_addr.parse()?;

    let listener = TcpListener::bind(addr).await?;

    let player = Arc::new(Player::new(Args::parse().ffplay_path.as_str()));
    let volume_controller = Arc::new(VolumeController::new());
    let autogrzybke = Arc::new(Autogrzybke::new(
        Args::parse().autogrzybke_resources_path.as_str(),
        Args::parse().prefix_chance_percent,
        Args::parse().suffix_chance_percent,
    ));

    let scheduler = Arc::new(
        Scheduler::new(
            player.clone(),
            Args::parse().autogrzybke_resources_path.as_str(),
        )
        .context("creating scheduler")
        .unwrap(),
    );

    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        let player = player.clone();
        let volume_controller = volume_controller.clone();
        let autogrzybke = autogrzybke.clone();
        let scheduler = scheduler.clone();
        let scheduler2 = scheduler.clone();

        tokio::task::spawn(async move {
            scheduler2.run_schedule().await;
        });

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(
                    io,
                    service_fn(move |request| {
                        http_request_handler::handle_request(
                            request,
                            player.clone(),
                            volume_controller.clone(),
                            autogrzybke.clone(),
                            scheduler.clone(),
                        )
                    }),
                )
                .await
            {
                error!("Error serving connection: {:?}", err);
            }
        });
    }
}
