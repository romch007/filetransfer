mod errors;

use std::{
    future,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::get,
    Router,
};
use bytes::Bytes;
use dashmap::DashMap;
use errors::{InternalErrExt, NotFoundExt};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Deserialize)]
struct Config {
    addr: Option<IpAddr>,
    port: Option<u16>,
}

#[derive(Debug)]
enum TransferState {
    Data(Bytes),
    End,
}

#[derive(Debug, Clone, Default)]
struct AppState {
    clients: Arc<DashMap<String, UnboundedSender<TransferState>>>,
}

async fn get_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let (bytes_sender, bytes_receiver) = tokio::sync::mpsc::unbounded_channel();
    state.clients.insert(file_id, bytes_sender);

    // Create a Stream from the UnboundedReceiver
    let transfer_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(bytes_receiver)
        // make the stream stop when reaching TransferState::End
        .take_while(|state| future::ready(!matches!(state, TransferState::End)))
        // create a Stream of Result<Bytes, _>
        .map(|state| match state {
            TransferState::Data(b) => anyhow::Ok(b),
            TransferState::End => unreachable!(),
        });

    let resp = axum::response::Response::builder()
        .body(Body::from_stream(transfer_stream))
        .map_internal_err()?;

    Ok(resp)
}

async fn put_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(), StatusCode> {
    let client = state.clients.get(&file_id).map_not_found()?;

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let Some(field_name) = field.name() else {
            continue;
        };

        if field_name == "file" {
            // Exhaust the multipart field
            while let Some(buf) = field.next().await {
                let buf = buf.map_internal_err()?;

                // Send bytes to connected client
                client.send(TransferState::Data(buf)).map_internal_err()?;
            }

            client.send(TransferState::End).map_internal_err()?;

            return Ok(());
        }
    }

    Err(StatusCode::UNPROCESSABLE_ENTITY)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = envy::from_env::<Config>().expect("cannot deserialize env");

    let sockaddr = SocketAddr::new(
        config.addr.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        config.port.unwrap_or(8080),
    );

    let state = AppState::default();
    let router = Router::new()
        .route("/{file_id}", get(get_file).post(put_file))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(sockaddr)
        .await
        .expect("cannot bind port");

    tracing::info!("listening on {sockaddr}");

    axum::serve(listener, router)
        .await
        .expect("cannot serve http");
}
