use crate::auth::AuthConnector;
use crate::column_range::ColumnRange;
use anyhow::Context;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Url;
use serde_json::{Map, Value};
use strum_macros::AsRefStr;
use yup_oauth2::authenticator::Authenticator;

/// The scopes required to access the Google Sheets API.
#[derive(AsRefStr, Debug)]
enum AccessScope {
    #[strum(to_string = "https://www.googleapis.com/auth/spreadsheets.readonly")]
    ReadOnly,
}

/// A client for interacting with the Google Sheets API.
pub struct GoogleSheetClient {
    client: reqwest::Client,
    base_url: String,
}

impl GoogleSheetClient {

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://sheets.googleapis.com".to_string(),
        }
    }

    pub fn with_base_url(base_url: Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Fetches rows from a spreadsheet and deserializes them into a typed collection.
    ///
    /// # Type Parameters
    /// * `T` - A type that implements `serde::Deserialize`.
    /// # Arguments
    /// * `auth` - Authenticator for accessing the Google Sheets API.
    /// * `spreadsheet_id` - Identifier of the Google Sheet to fetch.
    /// * `range` - Column range to fetch from the Google Sheet.
    pub async fn fetch_typed_rows<T>(
        &self,
        auth: &Authenticator<AuthConnector>,
        spreadsheet_id: &str,
        range: &ColumnRange,
    ) -> anyhow::Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        // Get the access token.
        let token = get_access_token(auth, AccessScope::ReadOnly).await?;

        // Fetch the Google Sheet as serde_json JSON (Value).
        let json_arrays = self
            .fetch_as_json(spreadsheet_id, range.to_api_string(), token)
            .await?;

        // Transform the serde_json JSON (Value) into type T.
        let typed_rows =
            transform_to_typed(json_arrays).context("Failed to deserialize Google Sheet")?;

        Ok(typed_rows)
    }

    /// Fetch data from a Google Sheet as JSON (serde_json::Value)
    async fn fetch_as_json<S1, S2, S3>(
        &self,
        spreadsheet_id: S1,
        range: S2,
        token: S3,
    ) -> anyhow::Result<Value>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
        S3: AsRef<str>,
    {
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
                    "Failed to fetch Google Sheet id {}, range {}",
                    spreadsheet_id.as_ref(),
                    range.as_ref(),
                )
            })?;

        // Convert the JSON response string as serde JSON (serde_json::Value)
        let mut json = response
            .json::<Value>()
            .await
            .context("Failed to parse Google Sheet JSON")?;

        // Get the `values` property from the JSON, which is a JSON array (rows) of arrays (cells in a row)
        let values = json
            .get_mut("values")
            .map(|v| v.take()) // Takes the value, leaving Null in its place
            .context("No values property found in Google Sheet JSON")?;

        Ok(values)
    }
}

async fn get_access_token(
    auth: &Authenticator<AuthConnector>,
    scope: AccessScope,
) -> anyhow::Result<String> {
    let scopes = &[scope.as_ref()];
    let token = auth
        .token(scopes)
        .await
        .context("Failed to get access token for Google Sheet fetch")?;

    let token = token
        .token()
        .context("Failed to retrieve access token string from Google Authenticator")?;

    Ok(String::from(token))
}

/// Return as type T instead of type Value
fn transform_to_typed<T>(json_arrays: Value) -> anyhow::Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    // Ensure that the JSON is an array of arrays
    let rows = match json_arrays {
        Value::Array(rows) => rows,
        _ => {
            return Err(anyhow::anyhow!(
                "Unexpected Google Sheet format, expected array of arrays"
            ))
        }
    };

    // Get an iterator over the rows
    let mut iter = rows.into_iter();

    // Deserialize the headers from the first row
    let Some(json_headers) = iter.next() else {
        return Err(anyhow::anyhow!("No rows found, is the Google Sheet empty?"));
    };

    // Convert headers to Vec<String>
    let headers: Vec<String> = serde_json::from_value::<Vec<String>>(json_headers)
        .context("Failed to deserialize headers from fetched Google Sheet")?;

    // Convert each row into a JSON object. Headers are the keys, and the corresponding cells are
    // the values
    let json_objects: Vec<Value> = iter
        .map(|mut row| {
            // row is an owned Value (an Array)
            let mut map = Map::new();
            if let Some(row_array) = row.as_array_mut() {
                for (header, cell_value) in headers.iter().zip(row_array.iter_mut()) {
                    // take() replaces the cell with Null and gives us the owned Value
                    map.insert(header.clone(), std::mem::take(cell_value));
                }
            }
            Value::Object(map)
        })
        .collect();

    // Deserialize the JSON objects into type T
    let typed_rows = serde_json::from_value(Value::Array(json_objects))
        .context("Failed to deserialize Google Sheet rows containing contact information")?;

    Ok(typed_rows)
}

#[cfg(test)]
mod tests {
}
