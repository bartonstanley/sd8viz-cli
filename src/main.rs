use anyhow::Context;
use clap::Parser;

mod auth;
mod contact_information;
mod googlesheet;
mod column_range;

use crate::contact_information::ContactInformation;
use crate::column_range::ColumnRange;
use auth::get_authenticator;
use contact_information::get_contact_information;

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

fn main() -> anyhow::Result<()> {
    if let Err(e) = run_app() {
        // Print the error message
        eprintln!("Error: {}", e);

        // Print the chain of causes in a more readable format than `:#` does.
        for cause in e.chain().skip(1) {
            eprintln!("  Caused by: {}", cause);
        }

        std::process::exit(1);
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn run_app() -> anyhow::Result<()> {
    let args = Args::parse();

    // 1. Get Auth (Imperative Shell)
    let authenticator = get_authenticator(&args.service_account_key).await?;
    let google_sheet_client = googlesheet::GoogleSheetClient::new();

    // 2. Get Data (Imperative Shell)
    let range = ColumnRange::try_from(&args.column_range)
        .with_context(|| format!("Failed to parse column_range option: {:?}", args.column_range))?;
    // Get the spreadsheet as Vec<ContactInformation>, where each row is a ContactInformation
    let rows: Vec<ContactInformation> =
        google_sheet_client.fetch_typed_rows(&authenticator, &args.sheet_id, &range).await?;
    println!("{:?}", rows);

    // 3. Get Shapefile (Imperative Shell)

    // 4. Process (Functional Core)
    // process_data(&rows)?;

    Ok(())
}

fn process_data(rows: &Vec<ContactInformation>) -> anyhow::Result<()> {
    // Get only the rows that have contact information
    // TODO: need an offset so error messages can give a row number
    let precinct_contact_rows = get_contact_information(&rows);
    precinct_contact_rows
        .iter()
        .for_each(|row| println!("row: {:?}", row));

    // validate and structure the rows of arrays of strings

    // convert structured rows to rows of contact counts per precinct

    // match contact counts per precinct to precinct geometry from shapefile

    // generate tiles

    Ok(())
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
