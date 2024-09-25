use futures::{stream::FuturesUnordered, StreamExt};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub mod city_stats_fetcher;
pub mod weather_fetcher;

// internal modules containing simple implementations for a couple public APIs
mod city_stats_api;
mod weather_api;

// We leverage thiserror (<https://docs.rs/thiserror/latest/thiserror/>), a handy macro
// that effectively automates some of the pain out of custom error types, especially the
// #[from] directive which reduces the amount of
// `.map_err(|e: SomeOtherCrateError| MyErrorType(SomeOtherCrateError))` calls you need
// to make
#[derive(Debug, Error)]
pub enum CityDataError {
    #[error("Data fetch failed with error: {0}")]
    FetchError(String),
    #[error("Handle send failed, mpsc dropped unexpectedly?")]
    HandleSendError(#[from] mpsc::error::SendError<CityDataRequest>),
    #[error("Handle recv failed, oneshot dropped unexpectedly?")]
    HandleRecvError(#[from] oneshot::error::RecvError),
    #[error("Task response send failed, oneshot droped unexpectedly?")]
    TaskSendError,
}

pub type CityDataResult<T> = Result<T, CityDataError>;

pub struct CityDataRequest {
    pub city: String,
    pub responder: oneshot::Sender<CityDataResult<String>>,
}

pub trait CityDataSource {
    /// Fetch city-specific data
    #[allow(async_fn_in_trait)] /* allow this as it's only used internally */
    async fn fetch_data(&self, city: String) -> CityDataResult<String>;
}

pub struct CityDataSourceHandle {
    pub data_request_sender: mpsc::Sender<CityDataRequest>,
}

impl CityDataSourceHandle {
    /// Request city-specific data
    ///
    /// # Errors
    /// If sending the request to the task or receiving a response fails
    pub async fn request_data(&self, city: String) -> CityDataResult<String> {
        let (responder, receiver) = oneshot::channel();
        let request = CityDataRequest { city, responder };

        self.data_request_sender.send(request).await?;

        receiver.await?
    }
}

// Note: For the most part the pub structs and impl fns below this comment only need to be pub(crate)
// instead of full pub. However, to be used in the ../tests directory they need to be pub, or at least
// behind some "testing" feature. I've opted to just make them pub for simplicity's sake, but an actual
// crate should do something smarter.

pub struct CityDataSourceTask<T>
where
    T: CityDataSource,
{
    data_source: T,
}

impl<T> CityDataSourceTask<T>
where
    T: CityDataSource,
{
    pub fn new(data_source: T) -> Self {
        Self { data_source }
    }

    async fn handle_request(&self, request: CityDataRequest) -> CityDataResult<()> {
        let city_data_result = self.data_source.fetch_data(request.city).await;

        request
            .responder
            .send(city_data_result)
            .map_err(|_| CityDataError::TaskSendError)
    }

    /// Run our task, looping on input from the `request_receiver` until its corresponding sender is dropped,
    /// or the `cancellation_token` is cancelled.
    ///
    /// Note: you may want to store `request_receiver` as a member of `self`. However, that creates a mutable
    /// reference issue where `request_receiver.recv()` requires a mutable reference to `request_receiver`,
    /// which would in turn require a mutable reference to `self`. This would then conflict with the various
    /// calls to `self.handle_input` which use immutable references to `self`, and in rust you can only hold
    /// one mutable reference xor one or more immutable references at a time.
    pub async fn run(
        &mut self,
        mut request_receiver: mpsc::Receiver<CityDataRequest>,
        cancellation_token: CancellationToken,
    ) {
        let mut request_pool = FuturesUnordered::new();

        loop {
            tokio::select! {
                request = request_receiver.recv() => {
                    let Some(request) = request else {
                        tracing::info!("task receiver dropped, shutting down");
                        break;
                    };
                    request_pool.push(self.handle_request(request));

                },
                Some(result) = request_pool.next(), if !request_pool.is_empty() => {
                    match result {
                        Ok(()) => {

                        },
                        Err(e) => {
                            tracing::error!("Data fetch failed with error {e:?}");
                        }
                    }
                },
                () = cancellation_token.cancelled() => {
                    tracing::info!("DataSourceTask cancellation token cancelled, shutting down");
                    break;
                }
            }
        }
    }
}
