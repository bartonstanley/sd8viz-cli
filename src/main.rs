mod auth;
mod column_range;
mod contact_information;
mod google_sheet_client;

use crate::auth::{get_access_token, AccessScope};
use crate::column_range::ColumnRange;
use crate::contact_information::{get_contact_information, ContactInformation};
use crate::google_sheet_client::GoogleSheetClient;
use anyhow::Context;
use clap::Parser;
use log;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Provide a username to trigger interactive browser login
    #[arg(short, long)]
    username: Option<String>,

    /// The ID of the Google Sheet to process
    #[arg(short = 'i', long)]
    sheet_id: String,

    #[arg(short = 'r', long, num_args = 3)]
    column_range: Vec<String>,

    /// Google Cloud Project ID where the secret is stored
    #[arg(long, env = "GCP_PROJECT_ID")]
    project_id: Option<String>,

    #[arg(short = 'k', long)]
    service_account_key: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    env_logger::init();

    // Get the access token.
    let access_token = get_access_token(&args.service_account_key, AccessScope::ReadOnly).await?;

    // Get google sheet client
    let google_sheet_client = GoogleSheetClient::new(access_token, None::<&str>);

    if let Err(e) = run_app(&args, &google_sheet_client).await {
        // Print the error message
        log::error!("Error: {}", e);

        // Print the chain of causes in a more readable format than `:#` does.
        for cause in e.chain().skip(1) {
            log::error!("  Caused by: {}", cause);
        }

        std::process::exit(1);
    }

    // Write the output to a file

    Ok(())
}

async fn run_app(
    args: &Args,
    google_sheet_client: &GoogleSheetClient,
) -> anyhow::Result<Vec<ContactInformation>> {
    // Get Data
    let range = ColumnRange::try_from(&args.column_range).with_context(|| {
        format!(
            "Failed to parse column_range option: {:?}",
            args.column_range
        )
    })?;

    // Get the spreadsheet as Vec<ContactInformation>, where each row is a ContactInformation
    let rows: Vec<ContactInformation> = google_sheet_client
        .fetch_typed_rows(&args.sheet_id, &range)
        .await?;

    // 3. Get Shapefile (Imperative Shell)

    // 4. Process (Functional Core)
    let precinct_contact_rows = process_data(&rows)?;

    Ok(precinct_contact_rows.into_iter().cloned().collect())
}

fn process_data(rows: &Vec<ContactInformation>) -> anyhow::Result<Vec<&ContactInformation>> {
    // Get only the rows that have contact information
    // TODO: need an offset so error messages can give a row number
    let precinct_contact_rows = get_contact_information(&rows);
    precinct_contact_rows
        .iter()
        .for_each(|row| println!("row: {:?}", row));

    // convert structured rows to rows of contact counts per precinct

    // match contact counts per precinct to precinct geometry from the shapefile

    // generate tiles

    Ok(precinct_contact_rows)
}

/*
use google_cloud_secretmanager_v1::client::SecretManagerService;
use yup_oauth2::ServiceAccountKey;
async fn get_service_account_key() -> anyhow::Result<ServiceAccountKey> {
    let client = SecretManagerService::builder().build().await?;
    println!("Fetching secret...");
    let response = client
        .access_secret_version()
        .set_name("projects/sd8viz-cli/secrets/SD8VIZ_CLI_CLIENT_SECRET/versions/latest")
        .send()
        .await?;
    let service_account_key =
        yup_oauth2::parse_service_account_key(response.payload.unwrap().data)?;
    println!("{:?}", service_account_key);

    Ok(service_account_key)
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::google_sheet_client::GoogleSheetClient;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_run_app_happy_path() {
        let args = get_fake_args();

        let mock_server = MockServer::start().await;
        let google_sheet_client = GoogleSheetClient::new(String::new(), Some(mock_server.uri()));

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({
            "values": [
                ["Precinct"],
                [""],
                ["2-01"],
                ["3-02"],
                [""],
            ]
        });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let rows = run_app(&args, &google_sheet_client).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].precinct, "2-01");
        assert_eq!(rows[1].precinct, "3-02");
    }

    #[tokio::test]
    async fn test_now_rows_in_sheet() {
        let args = get_fake_args();

        let mock_server = MockServer::start().await;
        let google_sheet_client = GoogleSheetClient::new(String::new(), Some(mock_server.uri()));

        // Define the mock response (Google Sheets API format)
        let response_body = serde_json::json!({ "values": [] });

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let result = run_app(&args, &google_sheet_client).await;

        let err = result.expect_err("Expected run to fail, but it did not");
        assert!(err
            .to_string()
            .starts_with("Failed to deserialize Google Sheet"));
    }

    fn get_fake_args() -> Args {
        Args {
            username: None,
            sheet_id: "1234567890".to_string(),
            column_range: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            project_id: None,
            service_account_key: String::new(),
        }
    }
}
