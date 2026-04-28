use anyhow::Context;
use strum_macros::AsRefStr;

/// The scopes required to access the Google Sheets API.
#[derive(AsRefStr, Debug)]
pub enum AccessScope {
    #[strum(to_string = "https://www.googleapis.com/auth/spreadsheets.readonly")]
    ReadOnly,
}

pub async fn get_access_token(
    service_account_key: &str,
    scope: AccessScope,
) -> anyhow::Result<String> {
    let service_account_key = yup_oauth2::read_service_account_key(service_account_key)
        .await
        .context("Failed to read service account key")?;
    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .context("Failed to build authenticator")?;

    let scopes = &[scope.as_ref()];
    let token = authenticator
        .token(scopes)
        .await
        .context("Failed to get access token")?;

    let token = token
        .token()
        .context("Failed to retrieve access token string from Google Authenticator")?;

    Ok(String::from(token))
}
