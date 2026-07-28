use reqwest::Client;

pub async fn get_content_length(client: &Client, url: &str) -> Result<u64, String> {
    let resp = client
        .head(url)
        .send()
        .await
        .map_err(|e| format!("HEAD failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HEAD returned {}", resp.status()));
    }

    resp.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "Missing Content-Length".into())
}

pub async fn download_image_streaming<F>(
    client: &Client,
    url: &str,
    on_progress: F,
) -> Result<(Vec<u8>, String, u64), String>
where
    F: Fn(u64, u64),
{
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

    let file_total = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let mut data = Vec::new();
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Read failed: {e}"))?;
        data.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        if file_total > 0 {
            on_progress(downloaded, file_total);
        }
    }

    Ok((data, ext.to_string(), file_total))
}
