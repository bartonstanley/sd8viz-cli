use anyhow::Context;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use yup_oauth2::authenticator::Authenticator;

pub type AuthConnector = HttpsConnector<HttpConnector>;

pub async fn get_authenticator() -> anyhow::Result<Authenticator<AuthConnector>> {
    let service_account_key =
        yup_oauth2::read_service_account_key("sd8viz-cli-firebase-adminsdk-fbsvc-ca6b0dbba5.json")
            .await
            .context("Failed to read service account key")?;
    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .context("Failed to build authenticator")?;

    Ok(authenticator)
}
