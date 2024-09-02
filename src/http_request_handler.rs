use crate::player::Player;
use crate::volume_controller::VolumeController;
use anyhow::anyhow;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, combinators::BoxBody, Empty, Full};
use hyper::body::Bytes;
use log::error;
use std::convert::Infallible;
use std::sync::Arc;
use url_encoded_data::UrlEncodedData;

pub async fn handle_request(
    request: Request<hyper::body::Incoming>,
    player: Arc<Player>,
    volume_controller: Arc<VolumeController>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(respond_with_root()),
        (&Method::POST, "/pause") => match player.pause() {
            Ok(_) => Ok(respond_with_root()),
            Err(err) => Ok(report_internal_server_error(err)),
        },
        (&Method::POST, "/play") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "stream_url"))
                .and_then(|url| player.play(url).map_err(|e| anyhow!(e)))
            {
                Ok(_) => Ok(respond_with_root()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }
        (&Method::POST, "/change_volume") => {
            match collect_request_body(request)
                .await
                .and_then(|b| get_value_from_form_body(b, "volume_delta"))
                .and_then(|vol| {
                    vol.parse::<i32>()
                        .map_err(|e| anyhow!(e).context("Parse volume_delta as int"))
                })
                .and_then(|vol| volume_controller.change_volume(vol))
            {
                Ok(_) => Ok(respond_with_root()),
                Err(err) => Ok(report_internal_server_error::<&dyn std::error::Error>(
                    err.as_ref(),
                )),
            }
        }

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
    #[error("Request body does not contain value for name \"{0}\"")]
    NoUrl(String),
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
        Some((_, v)) => {
            if v.is_empty() {
                Err(anyhow!(RequestBodyError::NoUrl(name.into())))
            } else {
                Ok(v.to_string())
            }
        }
        None => Err(anyhow!(RequestBodyError::NameNotFound(name.into()))),
    }
}
