use data_fetchers::{
    city_stats_fetcher::spawn_city_stats_fetcher_task, weather_fetcher::spawn_weather_fetcher_task,
    CityDataSourceHandle,
};
use futures::{stream::FuturesUnordered, StreamExt};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{info_span, Instrument};

#[derive(Debug, Error)]
pub enum DispatcherError {
    #[error("Failed to send request on mpsc, dropped unexpectedly?")]
    MpscSendFailed(#[from] mpsc::error::SendError<DispatcherRequest>),
    #[error("Failed to send response on oneshot, dropped unexpectedly?")]
    OneshotResponseFailed(#[from] oneshot::error::RecvError),
}

/// A custom `Response` type leveraging our `DispatcherError` above
pub type DispatcherResult<T> = Result<T, DispatcherError>;

/// A request to our `Dispatcher`
#[derive(Debug)]
pub struct DispatcherRequest {
    // the city our Dispatcher will aggregate info for
    city_name: String,
    // a oneshot channel to send the response
    response_sender: oneshot::Sender<DispatcherResponse>,
}

/// The response our Dispatcher will send
#[derive(Debug)]
struct DispatcherResponse {
    data: String,
}

/// The "Handle" we will pass out to anything that wishes to use the `Dispatcher`
/// Note that we can derive `Clone` because `mpsc::Sender` (multiple producer, single consumer)
/// impls `Clone`. Every clone of the sender sends messages to the same individual consumer
/// (the `Dispatcher`)
#[derive(Clone)]
pub struct DispatcherHandle {
    request_sender: mpsc::Sender<DispatcherRequest>,
}

impl DispatcherHandle {
    /// Get city-specific info from the dispatcher task
    ///
    /// # Errors
    /// If sending the request or receiving the response fails
    pub async fn get_city_info(&self, city_name: String) -> DispatcherResult<String> {
        let (response_sender, response_receiver) = oneshot::channel();
        let request = DispatcherRequest {
            city_name,
            response_sender,
        };

        // dispatch the request
        self.request_sender.send(request).await?;

        // wait for the response
        let response = response_receiver.await?.data;

        Ok(response)
    }
}

/// Handle a dispatcher request and send a response
async fn handle_request(request: DispatcherRequest, fetchers: &[CityDataSourceHandle]) {
    tracing::info!("Got request for city: {:?}", request.city_name);

    // Aggregate all fetcher responses
    let mut data = String::new();
    for f in fetchers {
        // Note: we could do this much more efficiently by using a `FuturesOrdered`
        // and generating all the requests "at once" before await-ing. This is left
        // as an exercise for the reader ;)
        let Ok(response) = f.request_data(request.city_name.clone()).await else {
            // if a single request fails, overwrite data and give up
            // Note: we could instead make `DispatcherResponse.data` a `Result<String>` so the
            // rest layer could more intelligently generate status codes, kept it this way for
            // simplicity
            data = String::from("Request failed");
            break;
        };

        data.push_str(&response);
        data.push('\n');
    }

    // ignore failures from the `response_sender`, this would only fail if the
    // corresponding `oneshot::Receiver` was dropped, in which case there's
    // nothing we can do here
    _ = request.response_sender.send(DispatcherResponse { data });
}

// The "Actor" loop, this is the thing which handles incoming requests
async fn run_dispatcher(
    cancellation_token: CancellationToken,
    mut receiver: mpsc::Receiver<DispatcherRequest>,
) {
    // Note: another option would be to have a vec of `Box<dyn dat_fetchers::CityDataSource>`, and directly call
    // `entry.fetch_data` for each entry in that Vec but that has a couple of disadvantages:
    // 1. Dynamic dispatch (`dyn` keyword) requires we use a `Box` which uses space on the stack and creates a vtable
    //    for function dispatch, which is slower. Standalone "Actor" tasks with handles act as "dynamic dispatch" in this way
    // 2. Every future created will be limited to this thread (due to the use of `tokio::select!`) where as standalone
    //    tasks can be executed in other threads
    let fetcher_handles: Vec<CityDataSourceHandle> = vec![
        spawn_city_stats_fetcher_task(cancellation_token.clone()),
        spawn_weather_fetcher_task(cancellation_token.clone()),
    ];

    // this FuturesUnordered is a pool of `Future`s you can treat like an async iterator, it will await
    // any futures it contains and `next` will return any completed future
    let mut pending_requests = FuturesUnordered::new();

    // who needs "while true"?
    loop {
        // note: tokio::select is a complex bit of async trickery and it is very much worth reading the docs
        // some important things to know:
        // - it will run until the first of the entries (which must be a future) completes, at which point it
        //   executes the associated branch
        // - you must use "cancel-safe" futures. in short: futures that don't store any state and can be `await`ed
        //   multiple times
        // - tokio::select limits execution to a single thread
        tokio::select! {
            optional_request = receiver.recv() => {
                // We recieved a message on our mpsc
                let Some(request) = optional_request else {
                    // mpsc returned None, this means all senders have been dropped. Given that the only senders
                    // are held by the rest API, that means something must have gone wrong, so we exit
                    tracing::warn!("request sender dropped, exiting");
                    break;
                };

                // push the request to the pending pool
                pending_requests.push(handle_request(request, &fetcher_handles));
            },
            _ = pending_requests.next(), if !pending_requests.is_empty() => {
                // nothing to actually do here, as `handle_request` isn't fallible, however we need this entry in the
                // select! statement so pending_requests is continuously polled, otherwise there will be nothing to
                // drive the futures it contains to completion.
                // note: we add the `if !empty()` check here so that the select! doesn't waste cycles getting `None`
                // back from `pending_requests.next()`. See the tokio::select! doc for more detail
            },
            () = cancellation_token.cancelled() => {
                // the parent cancellation token created in `main` was cancelled, meaning we've got to shut down
                tracing::info!("Task cancelled, exiting");
                break;
            }
        }
    }
}

/// Spawn our dispatcher inside a task, which will allow it to be scheduled on
/// Note: you may have noticed tha nowhere in this file is an actual `Dispatcher` struct. This is because we don't
/// actually have any state that we might want to store
pub fn spawn_dispatcher(cancellation_token: CancellationToken) -> DispatcherHandle {
    let (sender, receiver) = mpsc::channel(128);

    tokio::spawn(run_dispatcher(cancellation_token, receiver).instrument(info_span!("Dispatcher")));

    DispatcherHandle {
        request_sender: sender,
    }
}

#[cfg(test)]
mod tests {
    use data_fetchers::{CityDataRequest, CityDataSourceHandle};
    use tokio::sync::{mpsc, oneshot};

    use crate::{handle_request, DispatcherRequest, DispatcherResponse};

    fn make_test_fetcher() -> (CityDataSourceHandle, mpsc::Receiver<CityDataRequest>) {
        let (sender, receiver) = mpsc::channel(1);

        (
            CityDataSourceHandle {
                data_request_sender: sender,
            },
            receiver,
        )
    }

    fn make_test_request(
        city_name: String,
    ) -> (DispatcherRequest, oneshot::Receiver<DispatcherResponse>) {
        let (response_sender, response_receiver) = oneshot::channel();
        let test_request = DispatcherRequest {
            city_name,
            response_sender,
        };

        (test_request, response_receiver)
    }

    #[tokio::test]
    async fn test_handle_request() {
        let (test_fetcher_handle, mut test_fetcher_receiver) = make_test_fetcher();
        let test_fetchers = vec![test_fetcher_handle];

        let (test_request, mut response_receiver) =
            make_test_request(String::from("Unit Test City"));

        // spawn a "mock fetcher" task which receives data requests and sends
        // responses. Note: we do it this way as `handle_request` has multiple
        // `await` points, so it's easier to just let the fetcher verification
        // happen in the background
        tokio::spawn(async move {
            // we should see our fetcher receive a request for data
            let fetcher_request = test_fetcher_receiver
                .recv()
                .await
                .expect("Expected test_fetcher_sender not to be dropped");
            assert_eq!(fetcher_request.city, String::from("Unit Test City"));

            // now fire a response
            fetcher_request
                .responder
                .send(Ok(String::from("test data for Unit Test City")))
                .expect("expected to send a result");

            // now the task will exit, dropping the `test_fetcher_receiver`
        });

        // handle the request
        handle_request(test_request, &test_fetchers).await;

        // we should see a response on the receiver
        let response = response_receiver
            .try_recv()
            .expect("Expected to receive a dispatcher response");
        assert_eq!(
            response.data,
            String::from("test data for Unit Test City\n")
        );

        // if we send another request, it should fail as the "mock fetcher"
        // (intentionally) dropped the test_fetcher_receiver
        let (new_request, mut failed_response_receiver) =
            make_test_request(String::from("Broken Test Town"));

        // handle the request
        handle_request(new_request, &test_fetchers).await;

        // we should see a failed response on the receiver
        let response = failed_response_receiver
            .try_recv()
            .expect("Expected to receive a dispatcher response");
        assert_eq!(response.data, String::from("Request failed"));
    }
}
