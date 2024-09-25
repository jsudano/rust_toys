use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};
use dispatcher::DispatcherHandle;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct ApiState {
    dispatcher_handle: DispatcherHandle,
}

/// Start up the rest API task
///
/// # Errors
/// if the rest api task exits unexpectedly
pub async fn start_rest_api(
    dispatcher_handle: DispatcherHandle,
    cancellation_token: CancellationToken,
) -> anyhow::Result<()> {
    let router = setup_rest_app(dispatcher_handle);

    // run it with hyper
    let bind_address = String::from("127.0.0.1:4242");
    let listener = TcpListener::bind(bind_address).await?;
    Ok(axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            cancellation_token.cancelled().await;
            tracing::info!("REST API beginning graceful shutdown");
        })
        .await?)
}

fn setup_rest_app(dispatcher_handle: DispatcherHandle) -> Router {
    // build our application with a route
    Router::new()
        .route("/:city_name", get(get_city_info))
        // this state is passed to any path fn with the State() extractor
        .with_state(ApiState { dispatcher_handle })
}

/// Get city-specific info for the given city from our dispatcher
/// Note we return `(StatusCode, String)` here, which axum conveniently converts
/// into an HTTP response for us (<https://docs.rs/axum/latest/axum/response/index.html>)
async fn get_city_info(
    Path(city_name): Path<String>,
    State(state): State<ApiState>,
) -> (StatusCode, String) {
    tracing::info!("Querying data for city: {city_name}");

    // try to make the request, wrapping it in a timeout
    let Ok(result) = tokio::time::timeout(
        Duration::from_secs(10),
        state.dispatcher_handle.get_city_info(city_name),
    )
    .await
    else {
        // we timed out, return 408
        return (
            StatusCode::REQUEST_TIMEOUT,
            String::from("request timed out"),
        );
    };

    // Note: we could condense this and the timeout above into one match, but then you wind up with nested Result destructuring
    // in the match arms (like Ok(Ok(data)) => ...) which gets a little hard to read. Just a matter of preference
    let data = match result {
        Ok(data) => data,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}"));
        }
    };

    // All succeeded, return 200
    (StatusCode::OK, data)
}
