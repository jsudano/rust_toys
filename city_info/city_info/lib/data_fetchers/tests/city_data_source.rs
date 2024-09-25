use std::time::Duration;

use data_fetchers::{CityDataRequest, CityDataSource, CityDataSourceTask};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

struct TestDataSource;

// NOTE: This would be a fantastic application for an automock (https://docs.rs/mockall/latest/mockall/attr.automock.html)
// for now I've implemented a "mock" manually
impl CityDataSource for TestDataSource {
    async fn fetch_data(&self, city: String) -> data_fetchers::CityDataResult<String> {
        Ok(format!("Test result for {city}"))
    }
}

// NOTE: in reality, this could just be a unit test in `../src/lib.rs`, but I've put it here
// to show another approach to test organization. This approach is often used "system" or
// "module" tests that integrate bits from multiple modules in the lib
#[tokio::test]
async fn test_city_data_source_task() {
    let mut task = CityDataSourceTask::new(TestDataSource);
    let (request_sender, request_receiver) = mpsc::channel(1);
    let cancellation_token = CancellationToken::new();

    // start our task running
    let running_task_handle = tokio::spawn({
        let child_token = cancellation_token.clone();
        async move {
            task.run(request_receiver, child_token).await;
        }
    });

    // send a request in to our task
    let (response_sender, response_receiver) = oneshot::channel();
    request_sender
        .send(CityDataRequest {
            city: String::from("Module Test Hamlet"),
            responder: response_sender,
        })
        .await
        .expect("expected to send a request");

    // expect to get a valid response
    let response = response_receiver
        .await
        .expect("Expected to receive a response")
        .expect("Expected response not to be an error");
    assert_eq!(response, String::from("Test result for Module Test Hamlet"));

    // assert our task is still running happily
    assert!(!running_task_handle.is_finished());

    // now cancel our cancellation token and confirm the task shuts down
    cancellation_token.cancel();
    // need to yield back to tokio here so running_task_handle can be polled to detect
    // the token has been cancelled
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert!(running_task_handle.is_finished());
}
