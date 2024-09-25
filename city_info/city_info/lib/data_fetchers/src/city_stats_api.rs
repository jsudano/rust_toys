use std::fmt::Display;

use serde::Deserialize;

use crate::{CityDataError, CityDataResult};

const CITY_STATS_API_PATH: &str = "https://nominatim.openstreetmap.org/search?q=";
const CITY_STATS_API_ARGS: &str = "&format=json&limit=1"; // format response as json and limit to one result

fn request_path_for_city(city: &str) -> String {
    // replaces spaces with '+'
    let space_subbed_city = city.replace(' ', "+");

    format!("{CITY_STATS_API_PATH}{space_subbed_city}{CITY_STATS_API_ARGS}")
}

async fn query_city_api(
    http_client: &reqwest::Client,
    city_name: &str,
) -> CityDataResult<Vec<CityStatsResponse>> {
    http_client
        .get(request_path_for_city(city_name))
        .send()
        .await
        .map_err(|e| CityDataError::FetchError(e.to_string()))?
        .error_for_status()
        .map_err(|e| CityDataError::FetchError(e.to_string()))?
        .json::<Vec<CityStatsResponse>>()
        .await
        .inspect_err(|e| tracing::error!("Got error: {e:?}"))
        .map_err(|_| CityDataError::FetchError(String::from("deserialize failed")))
}

/// Fetches city statistics using the nominatim OSM API:
/// <https://nominatim.org/release-docs/latest/api/Search/>
pub(crate) async fn fetch_city_stats(
    http_client: &reqwest::Client,
    city_name: String,
) -> CityDataResult<String> {
    let city_stats_response = query_city_api(http_client, &city_name).await?;

    // Just grab the first result,
    let city_details = city_stats_response
        .first()
        .ok_or(CityDataError::FetchError(String::from("no city found")))?;

    Ok(city_details.to_string())
}

/// A struct representing a response from the nominatim OSM API
/// Note: the response contains much more data than this, but serde will selectively pick out fields
/// that match struct field names and ignore the rest
#[derive(Deserialize)]
struct CityStatsResponse {
    #[serde(rename = "display_name")]
    // look for a field in the input named "display_string" and populate this struct field with its contents
    city_county_state_country_str: String,
}

/// impl Display for `CityStatsResponse` so we can call `to_string()` (or throw it into `format!()`)
impl Display for CityStatsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Stats for {}:",
            self.city_county_state_country_str
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::city_stats_api::query_city_api;

    use super::CityStatsResponse;

    #[tokio::test]
    async fn test_query_api() {
        let client = reqwest::Client::builder()
            .user_agent("rust_toys_test")
            .build()
            .expect("Failed to build user agent!");

        query_city_api(&client, "San Jose").await.expect("WARNING: Failed to query or parse geocoding data for a known city, this means the API is not reachable or its response format has changed");
    }

    #[test]
    fn test_format() {
        let stats = CityStatsResponse {
            city_county_state_country_str: String::from("Unit Test City"),
        };

        let expected_format = String::from("Stats for Unit Test City:");

        assert_eq!(format!("{stats}"), expected_format);
        assert_eq!(stats.to_string(), expected_format);
    }
}
