use crate::autogrzybke::Autogrzybke;
use crate::benny::Benny;
use crate::player::Player;
use crate::schedule;
use crate::schedule::Scheduler;
use crate::volume_controller::VolumeController;
use anyhow::{anyhow, Context};
use http::{Method, Request, Response, StatusCode};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use log::{error, info};
use std::convert::Infallible;
use std::sync::Arc;
use url_encoded_data::UrlEncodedData;
use crate::resource_catalogue::ResourceCatalogue;

pub async fn handle_request(
    request: Request<hyper::body::Incoming>,
    player: Arc<Player>,
    volume_controller: Arc<VolumeController>,
    autogrzybke: Arc<Autogrzybke>,
    scheduler: Arc<Scheduler>,
    benny: Arc<Benny>,
    resources_catalogue: Arc<ResourceCatalogue>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(respond_with_root()),
        (&Method::POST, "/pause") => match player.pause() {
            Ok(_) => Ok(respond_ok()),
            Err(err) => Ok(report_internal_server_error(err)),
        },
        (&Method::POST, "/play") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "stream_url"))
                .and_then(|url| {
                    player
                        .play(url, chrono::Duration::seconds(0))
                        .map_err(|e| anyhow!(e))
                }) {
                Ok(_) => Ok(respond_ok()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::POST, "/playserverfiles") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "playlist"))
                .and_then(|s| Ok(s.trim().to_string()))
                .and_then(|missing| Ok(missing.split("\r\n").map(|slice| slice.into()).collect()))
                .and_then(|playlist| player.play_local_playlist(playlist).map_err(|e| anyhow!(e)))
            {
                Ok(_) => Ok(respond_ok()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::GET, "/listserverfiles") => {
            Ok(respond_with_html(resources_catalogue.get_joned_list_of_files().to_string()))
        }
        (&Method::POST, "/change_volume") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "volume_delta"))
                .and_then(|vol| {
                    if vol.is_empty() {
                        return Ok(0);
                    }
                    vol.parse::<i32>()
                        .map_err(|e| anyhow!(e).context("Parse volume_delta as int"))
                })
                .and_then(|vol| volume_controller.change_volume(vol))
            {
                Ok(_) => Ok(respond_ok()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::GET, "/autogrzybke") => {
            Ok(respond_with_autogrzybke(autogrzybke.get_last_missing()))
        }
        (&Method::POST, "/autogrzybke") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "missing"))
                .and_then(|missing| {
                    Ok(missing
                        .split_whitespace()
                        .map(|slice| slice.into())
                        .collect())
                })
                .and_then(|missing| Ok(autogrzybke.generate_playlist(missing)))
                .inspect(|playlist| {
                    info!("Generated playlist:\n{}", playlist.join("\n"));
                })
                .and_then(|playlist| player.play_local_playlist(playlist).map_err(|e| anyhow!(e)))
            {
                Ok(_) => Ok(respond_ok()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::GET, "/jukebox") => Ok(respond_with_jukebox()),
        (&Method::GET, "/autohypys") => {
            Ok(respond_with_schedule(scheduler.get_serialized_schedule()))
        }
        (&Method::POST, "/autohypys") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "schedule"))
                .and_then(|text| scheduler.set_schedule(text.as_str()))
            {
                Ok(_) => Ok(respond_with_schedule(scheduler.get_serialized_schedule())),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::POST, "/autohypys/reset") => match scheduler
            .set_schedule(schedule::SCHEDULE_DEFAULT)
            .context("Handle POST /autohypys/reset")
        {
            Ok(_) => Ok(respond_with_schedule(scheduler.get_serialized_schedule())),
            Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                err.as_ref(),
            )),
        },
        (&Method::POST, "/benny") => match benny.toggle() {
            Ok(_) => Ok(respond_ok()),
            Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                err.as_ref(),
            )),
        },
        _ => Ok(respond_not_found()),
    }
}

fn report_internal_server_error<E>(error: E) -> Response<BoxBody<Bytes, Infallible>>
where
    E: std::error::Error,
{
    error!("{error:?}");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::new(format!("{:?}", error).into()).boxed())
        .unwrap()
}

fn respond_with_html(html: String) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from(html)).boxed())
        .unwrap()
}

fn respond_with_root() -> Response<BoxBody<Bytes, Infallible>> {
    let html = include_str!("fosiaudio_chilli.html").to_string();
    respond_with_html(html)
}

fn respond_with_autogrzybke(missing: Vec<String>) -> Response<BoxBody<Bytes, Infallible>> {
    let html = include_str!("autogrzybke.html").to_string();
    let html = html.replace("LAST_MISSING", missing.join("\n").as_str());
    respond_with_html(html)
}

fn respond_with_jukebox() -> Response<BoxBody<Bytes, Infallible>> {
    let html = include_str!("jukebox.html").to_string();
    respond_with_html(html)
}

fn respond_with_schedule(
    schedule_text: Result<String, anyhow::Error>,
) -> Response<BoxBody<Bytes, Infallible>> {
    match schedule_text {
        Ok(text) => {
            let html = include_str!("autohypys.html").to_string();
            let html = html.replace("SCHEDULE", text.as_str());
            respond_with_html(html)
        }
        Err(e) => respond_with_html(format!("{e}")),
    }
}

fn respond_ok() -> Response<BoxBody<Bytes, Infallible>> {
    let mut response = Response::new(Empty::<Bytes>::new().boxed());
    *response.status_mut() = StatusCode::NO_CONTENT;
    return response.into();
}

fn respond_not_found() -> Response<BoxBody<Bytes, Infallible>> {
    let mut response = Response::new(Empty::<Bytes>::new().boxed());
    *response.status_mut() = StatusCode::NOT_FOUND;
    return response.into();
}

async fn collect_request_body(
    request: Request<hyper::body::Incoming>,
) -> Result<Bytes, anyhow::Error> {
    let bytes = request
        .into_body()
        .collect()
        .await
        .map_err(|e| anyhow!(e))?
        .to_bytes();
    Ok(bytes)
}

#[derive(thiserror::Error, Debug)]
pub enum RequestBodyError {
    #[error("Request body is empty. Expected: \"stream_url=<url>\"")]
    EmptyBody,
    #[error("Request body is not an utf8 string. Expected: \"stream_url=<url>\"")]
    NotAnUtf8Body(#[from] std::string::FromUtf8Error),
    #[error("Request body does not contain \"{0}\" name")]
    NameNotFound(String),
}

fn get_value_from_form_body(body: Bytes, name: &str) -> Result<String, anyhow::Error> {
    let chunk = body
        .utf8_chunks()
        .next()
        .ok_or(anyhow!(RequestBodyError::EmptyBody))
        .and_then(|chunk| Ok(chunk.valid()))?;
    match UrlEncodedData::parse_str(chunk)
        .iter()
        .find(|(k, _)| **k == name)
    {
        Some((_, v)) => Ok(v.to_string()),
        None => Err(anyhow!(RequestBodyError::NameNotFound(name.into()))),
    }
}
