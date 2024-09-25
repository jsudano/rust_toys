use std::fmt::Display;

use serde::Deserialize;

use crate::{CityDataError, CityDataResult};

const WEATHER_API_PATH: &str = "http://wttr.in/";
const WEATHER_API_ARGS: &str = "?format=j1";

fn request_path_for_city(city: &str) -> String {
    // drop all spaces
    let space_subbed_city = city.replace(' ', "");

    format!("{WEATHER_API_PATH}{space_subbed_city}{WEATHER_API_ARGS}")
}

async fn query_weather_api(
    http_client: &reqwest::Client,
    city_name: &str,
) -> CityDataResult<WeatherResponse> {
    http_client
        .get(request_path_for_city(city_name))
        .send()
        .await
        .map_err(|e| CityDataError::FetchError(e.to_string()))?
        .error_for_status()
        .map_err(|e| CityDataError::FetchError(e.to_string()))?
        .json::<WeatherResponse>()
        .await
        .inspect_err(|e| tracing::error!("Got error: {e:?}"))
        .map_err(|_| CityDataError::FetchError(String::from("deserialize failed")))
}

/// Fetches weather for a city using wttr.in
/// <https://github.com/chubin/wttr.in> (this is a super fun command line utility and you should try it!)
pub(crate) async fn fetch_weather_data(
    http_client: &reqwest::Client,
    city_name: String,
) -> CityDataResult<String> {
    let city_stats_response = query_weather_api(http_client, &city_name).await?;

    let entry = city_stats_response
        .current_condition
        .first()
        .ok_or(CityDataError::FetchError(String::from("no city found")))?;

    Ok(entry.to_string())
}

/// A struct representing the JSON response from wttr.in
/// Note: the response contains much more data than this, but serde will selectively pick out fields
/// that match struct field names and ignore the rest
#[derive(Deserialize)]
struct WeatherResponse {
    current_condition: Vec<WeatherEntry>,
}

#[derive(Deserialize)]
struct WeatherEntry {
    observation_time: String,
    #[serde(rename = "temp_C")]
    temp_c: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    #[serde(rename = "winddir16Point")]
    wind_dir_16_point: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WeatherDescription>,
}

#[derive(Deserialize, Default)]
struct WeatherDescription {
    value: String,
}

impl Display for WeatherEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Weather at {}: {}C (feels like {}C) and {} with winds from {} at {}kph",
            self.observation_time,
            self.temp_c,
            self.feels_like_c,
            self.weather_desc
                .first()
                .map_or("none", |d| d.value.as_ref()),
            self.wind_dir_16_point,
            self.windspeed_kmph
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::weather_api::query_weather_api;

    use super::{WeatherDescription, WeatherEntry};

    #[tokio::test]
    async fn test_query_api() {
        let client = reqwest::Client::builder()
            .user_agent("rust_toys_test")
            .build()
            .expect("Failed to build user agent!");

        query_weather_api(&client, "San Jose").await.expect("WARNING: Failed to query or parse geocoding data for a known city, this means the API is not reachable or its response format has changed");
    }

    #[test]
    fn test_format_response() {
        let entry = WeatherEntry {
            observation_time: String::from("10:09 PM"),
            temp_c: String::from("20"),
            feels_like_c: String::from("21"),
            wind_dir_16_point: String::from("ESE"),
            windspeed_kmph: String::from("12"),
            weather_desc: vec![WeatherDescription {
                value: String::from("Sunny"),
            }],
        };

        let expected_format = String::from(
            "Weather at 10:09 PM: 20C (feels like 21C) and Sunny with winds from ESE at 12kph",
        );

        assert_eq!(format!("{entry}"), expected_format);
        assert_eq!(entry.to_string(), expected_format);
    }
}
