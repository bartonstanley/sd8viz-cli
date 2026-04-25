use crate::auth::AuthConnector;
use anyhow::Context;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Map, Value};
use strum_macros::AsRefStr;
use yup_oauth2::authenticator::Authenticator;

pub struct SheetRange {
    sheet_name: String,

    // These are String and not char because columns after "Z" have multiple letters, e.g. "AA", "AB", "AC"
    start_col: String,
    end_col: String,
}

#[derive(AsRefStr, Debug)]
enum AccessScope {
    #[strum(to_string = "https://www.googleapis.com/auth/spreadsheets.readonly")]
    ReadOnly,
}

impl SheetRange {
    /// Creates a new range for a specific sheet and column span.
    pub fn new(sheet_name: &str, start_col: &str, end_col: &str) -> Self {
        Self {
            sheet_name: sheet_name.to_string(),
            start_col: start_col.to_string(),
            end_col: end_col.to_string(),
        }
    }

    /// Internal helper to format and URL-encode the range for the API.
    /// This is private because the API client shouldn't care about the string format.
    fn to_api_string(&self) -> String {
        let raw_range = format!("{}!{}:{}", self.sheet_name, self.start_col, self.end_col);
        // Use urlencoding to handle spaces and special characters safely
        urlencoding::encode(&raw_range).into_owned()
    }
}

pub struct GoogleSheetClient {
    client: reqwest::Client,
}

impl GoogleSheetClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_typed_rows<T>(
        &self,
        auth: &Authenticator<AuthConnector>,
        spreadsheet_id: &str,
        range: &SheetRange,
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
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
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

        let mut json = response
            .json::<Value>()
            .await
            .context("Failed to parse Google Sheet JSON")?;

        let values = json
            .get_mut("values")
            .map(|v| v.take()) // Takes the value, leaving Null in its place
            .context("No data found")?;

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
        .context("Failed to get access token from Google Authenticator")?;

    Ok(String::from(token))
}

/// Return as type T instead of Value
fn transform_to_typed<T>(json_arrays: Value) -> anyhow::Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    // Ensure that the JSON is an array of arrays
    let rows = match json_arrays {
        Value::Array(rows) => rows,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid Google Sheet format: expected array of arrays"
            ))
        }
    };

    // Get an iterator over the rows
    let mut iter = rows.into_iter();

    // Deserialize the headers from the first row
    let Some(json_headers) = iter.next() else {
        return Err(anyhow::anyhow!("No data found"));
    };

    let headers: Vec<String> = serde_json::from_value::<Vec<String>>(json_headers)
        .context("Failed to deserialize headers from fetched Google Sheet")?;

    let json_objects: Vec<Value> = iter
        .map(|mut row| {
            // row is now owned Value (an Array)
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
