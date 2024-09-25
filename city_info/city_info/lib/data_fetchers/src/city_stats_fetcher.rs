use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    city_stats_api::fetch_city_stats, CityDataResult, CityDataSource, CityDataSourceHandle,
    CityDataSourceTask,
};

pub struct CityStatsFetcher {
    // An http client we can re-use to avoid re-initializing TLS stuff
    // and do connection pooling
    http_client: reqwest::Client,
}

impl CityStatsFetcher {
    fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("rust_toys_test") // this API requires a user-agent for usage tracking
            .build()
            // in the interest of simplicity, we use `expect()` which will panic if we fail to build the
            // client. This should almost always be avoided in production code, but is fine here as
            // build() should rarely fail for our use case
            .expect("Failed to build user agent!");
        Self { http_client }
    }
}

impl CityDataSource for CityStatsFetcher {
    async fn fetch_data(&self, city: String) -> CityDataResult<String> {
        fetch_city_stats(&self.http_client, city).await
    }
}

pub fn spawn_city_stats_fetcher_task(
    cancellation_token: CancellationToken,
) -> CityDataSourceHandle {
    let fetcher = CityStatsFetcher::new();
    let (sender, receiver) = mpsc::channel(16);

    tokio::spawn(async move {
        let mut task = CityDataSourceTask::new(fetcher);
        task.run(receiver, cancellation_token).await;
    });

    CityDataSourceHandle {
        data_request_sender: sender,
    }
}
