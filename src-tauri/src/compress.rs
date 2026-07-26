use image::DynamicImage;

pub fn compress(bytes: &[u8], max_mb: f64) -> Vec<u8> {
    let max_bytes = (max_mb * 1_000_000.0) as usize;
    if bytes.len() <= max_bytes {
        return bytes.to_vec();
    }

    let img = match image::load_from_memory(bytes) {
        Ok(img) => img,
        Err(_) => return bytes.to_vec(),
    };

    let qualities = [85u8, 70, 55];
    for q in qualities {
        let out = encode_jpeg(&img, q);
        if out.len() <= max_bytes {
            return out;
        }
    }

    let scales = [0.85, 0.7, 0.55, 0.4];
    for &scale in &scales {
        let w = (img.width() as f64 * scale) as u32;
        let h = (img.height() as f64 * scale) as u32;
        let resized = img.resize_exact(w.max(1), h.max(1), image::imageops::FilterType::Lanczos3);
        for q in [80u8, 65] {
            let out = encode_jpeg(&resized, q);
            if out.len() <= max_bytes {
                return out;
            }
        }
    }

    let w = (img.width() as f64 * 0.3) as u32;
    let h = (img.height() as f64 * 0.3) as u32;
    let final_img = img.resize_exact(w.max(1), h.max(1), image::imageops::FilterType::Lanczos3);
    encode_jpeg(&final_img, 60)
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    if encoder.encode_image(img).is_err() {
        return vec![];
    }
    out
}
