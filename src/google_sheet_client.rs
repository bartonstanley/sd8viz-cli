use crate::column_range::ColumnRange;
use anyhow::{bail, Context};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Map, Value};
use std::vec::IntoIter;

/// A client for interacting with the Google Sheets API.
pub struct GoogleSheetClient {
    access_token: String,
    base_url: String,
    client: reqwest::Client,
}

impl GoogleSheetClient {
    pub fn new<S: Into<String>>(access_token: String, base_url: Option<S>) -> Self {
        let base_url = base_url
            .map(|s| s.into())
            .unwrap_or_else(|| "https://sheets.googleapis.com".to_string());
        Self {
            access_token,
            base_url,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap(),
        }
    }

    /// Fetches rows from a spreadsheet and deserializes them into a typed collection.
    ///
    /// # Type Parameters
    /// * `T` - A type that implements `serde::Deserialize`.
    /// # Arguments
    /// * `spreadsheet_id` - Identifier of the Google Sheet to fetch.
    /// * `range` - Column range to fetch from the Google Sheet.
    pub async fn fetch_typed_rows<T>(
        &self,
        spreadsheet_id: &str,
        range: &ColumnRange,
    ) -> anyhow::Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        // Fetch the Google Sheet as serde_json JSON (Value).
        let json_arrays = self
            .fetch_as_json(spreadsheet_id, range.to_api_string(), &self.access_token)
            .await?;

        // Transform the serde_json JSON (Value) into type T.
        let typed_rows =
            transform_to_typed(json_arrays).context("Failed to deserialize Google Sheet")?;

        Ok(typed_rows)
    }

    /// Fetch data from a Google Sheet as JSON (serde_json::Value)
    async fn fetch_as_json(
        &self,
        spreadsheet_id: impl AsRef<str>,
        range: impl AsRef<str>,
        token: impl AsRef<str>,
    ) -> anyhow::Result<Value> {
        // Construct the URL for the Google Sheets API
        let url = format!(
            "{}/v4/spreadsheets/{}/values/{}",
            self.base_url,
            spreadsheet_id.as_ref(),
            range.as_ref()
        );

        // Make the request to the Google Sheets API and check for errors
        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", token.as_ref()))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failure sending request to fetch Google Sheet id {}, range {}",
                    spreadsheet_id.as_ref(),
                    range.as_ref(),
                )
            })?;

        // Check for HTTP status code 2xx
        if !response.status().is_success() {
            bail!(
                "Unsuccessful return code {} fetching Google Sheet id {}, range {}",
                response.status(),
                spreadsheet_id.as_ref(),
                range.as_ref(),
            );
        }

        // Convert the JSON response string as serde JSON (serde_json::Value)
        let mut json = response.json::<Value>().await.with_context(|| {
            format!(
                "Failed to parse JSON of Google Sheet id {}, range {}",
                spreadsheet_id.as_ref(),
                range.as_ref(),
            )
        })?;

        // Get the `values` property from the JSON, which is a JSON array (rows) of arrays (cells in a row)
        let values = json
            .get_mut("values")
            .map(|v| v.take()) // Takes the value, leaving Null in its place
            .context("No values property found in Google Sheet JSON")?;

        Ok(values)
    }
}

/// Return as Vec<T> instead of Value
fn transform_to_typed<T>(json_array_of_arrays: Value) -> anyhow::Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    let rows = vet_outermost_json(json_array_of_arrays)?;

    let (headers, row_iter) = extract_headers_from_rows(rows)?;

    // Convert each row into a JSON object. Headers are the keys, and the corresponding cells are
    // the values
    let json_objects: Vec<Value> = row_iter
        .filter_map(|row| {
            // Vet the row using the new function
            let Some((row_vec, padding_count)) = vet_row(&row, &headers) else {
                return None;
            };

            // Create the padded iterator using repeat_n for efficiency
            let padded_row_iter = row_vec.into_iter().chain(std::iter::repeat_n(
                Value::String(String::new()),
                padding_count,
            ));

            // Zip with headers to create the JSON object map
            let map: Map<String, Value> = headers.iter().cloned().zip(padded_row_iter).collect();

            Some(Value::Object(map))
        })
        .collect();

    // Deserialize the JSON objects into type T
    let typed_rows = serde_json::from_value(Value::Array(json_objects))
        .context("Failed to deserialize Google Sheet rows containing contact information")?;

    Ok(typed_rows)
}

/// Ensure that the outermost JSON level is an array (individual row arrays will be checked later)
fn vet_outermost_json(json_array_of_arrays: Value) -> anyhow::Result<Vec<Value>> {
    let rows = match json_array_of_arrays {
        Value::Array(rows) => rows,
        _ => {
            return Err(anyhow::anyhow!(
                "Unexpected Google Sheet format, expected array"
            ))
        }
    };

    Ok(rows)
}

/// Extract headers as Vec<String> and return them with an iterator over the remaining rows.
fn extract_headers_from_rows(rows: Vec<Value>) -> anyhow::Result<(Vec<String>, IntoIter<Value>)> {
    // Get an iterator over the rows
    let mut row_iter = rows.into_iter();

    // Deserialize the headers from the first row
    let Some(json_headers) = row_iter.next() else {
        return Err(anyhow::anyhow!("No rows found, is the Google Sheet empty?"));
    };

    // Convert headers to Vec<String>
    let headers: Vec<String> = serde_json::from_value::<Vec<String>>(json_headers)
        .context("Failed to deserialize headers from fetched Google Sheet")?;

    Ok((headers, row_iter))
}

/// Vets a row by ensuring it is an array and checking for extra values compared to headers.
/// Issues a warning if the row contains more cells than there are headers.
fn vet_row(row: &Value, headers: &[String]) -> Option<(Vec<Value>, usize)> {
    // 1. Ensure the row is an array, otherwise default to an empty one
    let row_array = match row.as_array() {
        Some(arr) => arr.clone(),
        None => {
            log::warn!("Row is not an array, will be ignored.");
            return None
        },
    };

    let row_len = row_array.len();
    let header_len = headers.len();

    // 2. Check for extra values
    if row_len > header_len {
        log::warn!(
            "Warning: Row has {} values, but only {} headers defined. Extra values will be ignored.",
            row_len, header_len
        );
    }

    // Calculate padding: Use saturating_sub to safely get 0 if row_len >= header_len
    let padding_count = header_len.saturating_sub(row_len);

    Some((row_array, padding_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column_range::ColumnRange;
    use crate::contact_information::ContactInformation;
    use serde::Deserialize;
    use tracing_test::traced_test;
    use wiremock::matchers::{header, method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestRow {
        name: String,
        value: String,
    }

    #[test]
    fn test_new_with_none() {
        let client = GoogleSheetClient::new(String::new(), None::<&str>);
        assert_eq!(client.base_url, "https://sheets.googleapis.com");
    }

    #[test]
    fn test_new_with_argument() {
        let client = GoogleSheetClient::new(String::new(), Some("https://test.com"));
        assert_eq!(client.base_url, "https://test.com");
    }

    #[tokio::test]
    async fn test_fetch_as_json_happy_path() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;

        let spreadsheet_id = "test-id";
        let range = "Sheet1!A:B";
        let token = "test-token";

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "values": [
                ["name", "value"],
                ["item1", "100"]
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/v4/spreadsheets/.+/values/.+"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let client = get_test_client(&mock_server.uri()).await;

        // Execute the private method.
        let result = client
            .fetch_as_json(spreadsheet_id, range, token)
            .await
            .unwrap();

        assert_eq!(result.as_array().unwrap().len(), 2);
        assert_eq!(result[0][0], "name");
        assert_eq!(result[1][1], "100");
    }

    #[tokio::test]
    async fn test_fetch_typed_rows_happy_path() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let client = get_test_client(&mock_server.uri()).await;

        let spreadsheet_id = "test-id";
        let range = ColumnRange::new("Sheet1", "A", "B");

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "values": [
                ["Precinct"],
                ["2-01"],
                ["3-02"],
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        // Execute the private method.
        let rows: Vec<ContactInformation> = client
            .fetch_typed_rows(spreadsheet_id, &range)
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ContactInformation::new("2-01"));
        assert_eq!(rows[1], ContactInformation::new("3-02"));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_typed_rows_long_row() {
        tracing_log::LogTracer::init().ok();
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let client = get_test_client(&mock_server.uri()).await;

        let spreadsheet_id = "test-id";
        let range = ColumnRange::new("Sheet1", "A", "B");

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "values": [
                ["Precinct", "Status"],
                ["2-01"],
                ["2-02", "xyz"],
                ["3-02", "abc", "def"],
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        // Execute the private method.
        let _: Vec<ContactInformation> = client
            .fetch_typed_rows(spreadsheet_id, &range)
            .await
            .unwrap();

        logs_assert(|lines| {
            if lines.len() != 1 {
                return Err(format!("Expected 1 log lines, found {}", lines.len()));
            }
            if !lines[0].contains("Extra values will be ignored") {
                return Err("Warning log was missing 'Extra values will be ignored'".to_string());
            }
            Ok(())
        });
    }

    #[tokio::test]
    async fn test_failure_sending_request() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let spreadsheet_id = std::hint::black_box("test-id");

        // let range = "Sheet1!A:B";
        let range = ColumnRange::new("Sheet1", "A", "B");

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/", mock_server.uri())),
            )
            .expect(2..=10)
            .mount(&mock_server)
            .await;

        // Create a client pointing to the mock server
        let client = get_test_client(&mock_server.uri()).await;

        // Execute the private method.
        let result: anyhow::Result<Vec<ContactInformation>> = client
            .fetch_typed_rows(spreadsheet_id, &range)
            .await;

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .starts_with("Failure sending request to fetch Google Sheet"));
    }

    #[tokio::test]
    async fn test_non_200_http_response() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let spreadsheet_id = std::hint::black_box("test-id");
        let range = "Sheet1!A:B";
        let token = "test-token";

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        // Create a client pointing to the mock server
        let client = get_test_client(&mock_server.uri()).await;

        // Execute the private method.
        let result = client.fetch_as_json(spreadsheet_id, range, token).await;

        let err = result.unwrap_err();
        assert!(err.to_string().starts_with("Unsuccessful return code"));
    }

    #[tokio::test]
    async fn test_non_json_response() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let spreadsheet_id = "test-id";
        let range = "Sheet1!A:B";
        let token = "test-token";

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not JSON"))
            .mount(&mock_server)
            .await;

        // Create a client pointing to the mock server
        let client = get_test_client(&mock_server.uri()).await;

        // Execute the private method.
        let result = client.fetch_as_json(spreadsheet_id, range, token).await;

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .starts_with("Failed to parse JSON of Google Sheet"));
    }

    #[tokio::test]
    async fn test_no_values_property_in_json_response() {
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let spreadsheet_id = "test-id";
        let range = "Sheet1!A:B";
        let token = "test-token";

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "not-values-property": [
                ["name", "value"],
                ["item1", "100"]
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        // Create a client pointing to the mock server
        let client = get_test_client(&mock_server.uri()).await;

        // Execute the private method.
        let result = client.fetch_as_json(spreadsheet_id, range, token).await;

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .starts_with("No values property found in Google Sheet JSON"));
    }

    #[test]
    fn test_transform_to_typed() {
        let input = serde_json::json!([["name", "value"], ["Alice", "Alpha"], ["Bob", "Beta"]]);

        let result: Vec<TestRow> = transform_to_typed(input).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Alice");
        assert_eq!(result[0].value, "Alpha");
        assert_eq!(result[1].name, "Bob");
        assert_eq!(result[1].value, "Beta");
    }

    #[test]
    fn test_transform_to_typed_not_json_array() {
        let input = serde_json::json!({"z": "y"});

        let result: Result<Vec<TestRow>, _> = transform_to_typed(input);

        let err = result.unwrap_err();
        let error = err.to_string();
        log::error!("Error transforming to typed: {}", error);
        assert!(err
            .to_string()
            .starts_with("Unexpected Google Sheet format, expected array"));
    }

    #[test]
    fn test_transform_to_typed_empty_array() {
        let input = serde_json::json!([]);

        let result: Result<Vec<TestRow>, _> = transform_to_typed(input);

        let err = result.expect_err("Expect Err Result when payload is not JSON array, but got Ok");
        assert!(err
            .to_string()
            .starts_with("No rows found, is the Google Sheet empty?"));
    }

    #[test]
    fn test_transform_to_typed_no_headers() {
        let input = serde_json::json!([[{"no": "headers"}], ["Alice", "Alpha"], ["Bob", "Beta"]]);

        let result: Result<Vec<TestRow>, _> = transform_to_typed(input);

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .starts_with("Failed to deserialize headers from fetched Google Sheet"));
    }

    #[test]
    fn test_transform_to_typed_invalid_json_cell() {
        let input = serde_json::json!([["header1", "header2"], ["Alice", {"Alpha": "Beta"}], ["Bob", "Beta"]]);

        let result: Result<Vec<TestRow>, _> = transform_to_typed(input);

        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .starts_with("Failed to deserialize Google Sheet rows containing contact information"));
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestRowOptional {
        name: String,
        value: Option<String>,
    }

    #[test]
    fn test_transform_to_typed_empty_values() {
        let input = serde_json::json!([["name", "value"], ["Alice"]]);

        let result: Vec<TestRowOptional> = transform_to_typed(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Alice");
        assert_eq!(result[0].value, Some("".to_string()));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_row_failed_vetting() {
        tracing_log::LogTracer::init().ok();
        // Start a background HTTP mock server
        let mock_server = MockServer::start().await;
        let client = get_test_client(&mock_server.uri()).await;

        let spreadsheet_id = "test-id";
        let range = ColumnRange::new("Sheet1", "A", "B");

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "values": [
                ["Precinct", "Status"],
                ["2-01"],
                {},
                ["3-02", "abc"],
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        // Execute the private method.
        let _: Vec<ContactInformation> = client
            .fetch_typed_rows(spreadsheet_id, &range)
            .await
            .unwrap();

        logs_assert(|lines| {
            if lines.len() != 1 {
                return Err(format!("Expected 1 log lines, found {}", lines.len()));
            }
            if !lines[0].contains("Row is not an array, will be ignored.") {
                return Err("Warning log was missing 'Row is not an array, will be ignored.'".to_string());
            }
            Ok(())
        });
    }


    async fn get_test_client(mock_server_uri: &str) -> GoogleSheetClient {
        // Pass the mock URI directly to the constructor
        GoogleSheetClient::new(String::new(), Some(mock_server_uri))
    }
}
