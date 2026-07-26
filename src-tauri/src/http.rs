use reqwest::Client;
use std::time::Duration;

pub fn create_client(proxy: &str) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("PixivAndroidApp/5.0.234 (Android 9.0; PixBox)")
        .danger_accept_invalid_certs(false);

    if !proxy.is_empty() {
        let proxy = reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| e.to_string())
}

pub fn create_client_with_referer(proxy: &str) -> Result<Client, String> {
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("PixivAndroidApp/5.0.234 (Android 9.0; PixBox)")
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::REFERER,
                reqwest::header::HeaderValue::from_static("https://www.pixiv.net"),
            );
            headers
        });

    if !proxy.is_empty() {
        let proxy = reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| e.to_string())
}
