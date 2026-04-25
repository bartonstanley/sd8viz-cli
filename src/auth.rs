use anyhow::Context;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use yup_oauth2::authenticator::Authenticator;

pub type AuthConnector = HttpsConnector<HttpConnector>;

pub async fn get_authenticator(service_account_key: &str) -> anyhow::Result<Authenticator<AuthConnector>> {
    let service_account_key =
        yup_oauth2::read_service_account_key(service_account_key)
            .await
            .context("Failed to read service account key")?;
    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .context("Failed to build authenticator")?;

    Ok(authenticator)
}
