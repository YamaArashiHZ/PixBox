use reqwest::Client;

pub async fn download_image(client: &Client, url: &str) -> Result<(Vec<u8>, String), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download returned {}", resp.status()));
    }

    let ext = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            if ct.contains("png") {
                "png"
            } else if ct.contains("gif") {
                "gif"
            } else {
                "jpg"
            }
        })
        .unwrap_or("jpg");

    let data = resp
        .bytes()
        .await
        .map_err(|e| format!("Read failed: {e}"))?;

    Ok((data.to_vec(), ext.to_string()))
}
