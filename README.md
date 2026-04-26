# sd8viz-cli

Create PMTiles resources for sd8viz front ends

## Dev setup
Before running the app, run this command:
```aiignore
gcloud auth application-default login
```
You will be prompted to authenticate with Google Cloud. Enter the credentials for the SD8 account.
When this completes successfully, the app's auth will succeed.

A command-line tool designed to process Google Sheets data and synchronize it with geographic shapefiles to generate visualization tiles (MVT/PMTiles).

## Features

- **Google Sheets Integration**: Fetch data directly from Google Sheets using Service Account authentication.
- **Type-Safe Mapping**: Automatically maps spreadsheet columns to Rust structures.
- **Data Processing**: (In Progress) Validates contact information and calculates precinct-level metrics.
- **Geospatial Visualization**: (Planned) Integrates with Shapefiles to produce map tiles.

## Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable version)
- A Google Cloud Project with the **Google Sheets API** enabled.
- A **Service Account Key** (JSON format) with access to your target spreadsheet.

## Installation

```bash
git clone https://github.com/your-username/sd8viz-cli.git
cd sd8viz-cli
cargo build --release
```
## Usage

The tool requires a path to your Google Service Account key, the Spreadsheet ID, and the range to process.

```bash
./target/release/sd8viz-cli \
  --service-account-key path/to/key.json \
  --sheet-id "YOUR_SPREADSHEET_ID" \
  --column-range "Sheet1" "A" "G"
```

Arguments

| Flag | Long Flag | Description |
|------|-----------|-------------|
| -k | --service-account-key | Path to the Google Service Account JSON key. |
| -i | --sheet-id | The unique ID of the Google Sheet. |
| -r | --column-range | Takes 3 arguments: SheetName, StartColumn, EndColumn. |
|      | --project-id | (Optional) Google Cloud Project ID. |

## Development

### Project Structure

- `src/main.rs`: Application entry point and CLI parsing.
- `src/auth.rs`: Google OAuth2 authentication logic.
- `src/googlesheet.rs`: API client for fetching and parsing spreadsheet data.
- `src/contact_information.rs`: Data models and filtering logic for contact records.

### Testing

```aiignore
cargo test
```