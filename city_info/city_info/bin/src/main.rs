use std::{process::ExitCode, time::Duration};

use dispatcher::spawn_dispatcher;
use rest_api::start_rest_api;
use tokio::signal::unix::SignalKind;
use tokio_util::sync::CancellationToken;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> ExitCode {
    // setup a tracing subscriber to route our process logs to stdout
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // no ansi colors
        .with_thread_ids(true) // thread ids included in logs
        .with_target(false) // no target
        .with_file(true) // filename in logs
        .with_line_number(true) // line number in logs
        .with_filter(LevelFilter::INFO); // info level logs and above

    tracing_subscriber::Registry::default()
        .with(stdout_layer)
        .init();

    tracing::info!("city_info server starting up");

    // set up a parent level cancellation token
    let parent_token = CancellationToken::new();

    // start the dispatcher task running
    let dispatcher_handle = spawn_dispatcher(parent_token.clone());

    // start the http_server task running and pass it the dispatcher handle so it can send requests
    let api_task = start_rest_api(dispatcher_handle, parent_token.clone());

    // listen for ctrl+c and sigterm
    let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate())
        .expect("Failed to setup sigterm handler");
    let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())
        .expect("Failed to setup sigint handler");

    // Let the API task run until it exits (which it should never do) or the process is terminated externally
    let mut graceful_shutdown = true;
    tokio::select! {
        task_result = api_task =>  {
            tracing::error!("rest API exited unexpectedly with result: {task_result:?}");
            graceful_shutdown = false;
        },
        _ = sigterm.recv() => {
            tracing::info!("Shutting down gracefully");
        }
        _ = sigint.recv() => {
            tracing::info!("Shutting down gracefully");
        }
    }

    // Cancel our cancellation token and wait for any tasks to shutdown
    // Note: we could keep track of the `JoinHandle`s to the dispatcher and rest tasks, and wait for those to
    // exit instead of just sleeping. That is left as an exercise for the reader.
    parent_token.cancel();
    tokio::time::sleep(Duration::from_secs(2)).await;

    if graceful_shutdown {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
