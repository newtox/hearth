use crate::{error::AppError, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::header,
    response::Response,
};
use image::{
    codecs::{gif::{GifDecoder, GifEncoder, Repeat}, webp::WebPDecoder},
    imageops::FilterType,
    AnimationDecoder, DynamicImage, Frame, ImageFormat, RgbaImage,
};
use std::io::Cursor;

const SIZES: [u32; 4] = [32, 64, 128, 256];
const MAX_FRAMES: usize = 400;

fn scaled(w: u32, h: u32, size: u32) -> (u32, u32) {
    let s = size as f64 / w.min(h).max(1) as f64;
    if s >= 1.0 {
        return (w, h);
    }
    (((w as f64) * s).round().max(1.0) as u32, ((h as f64) * s).round().max(1.0) as u32)
}

fn resize(img: &RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = scaled(img.width(), img.height(), size);
    if (w, h) == img.dimensions() {
        return img.clone();
    }
    image::imageops::resize(img, w, h, FilterType::Lanczos3)
}

fn animate(frames: Vec<Frame>, size: u32) -> Option<Vec<u8>> {
    if frames.len() < 2 || frames.len() > MAX_FRAMES {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.set_repeat(Repeat::Infinite).ok()?;
        let resized = frames.into_iter().map(|f| {
            let delay = f.delay();
            Frame::from_parts(resize(f.buffer(), size), 0, 0, delay)
        });
        enc.encode_frames(resized).ok()?;
    }
    Some(out)
}

fn render(data: &[u8], ext: &str, size: u32) -> Option<(Vec<u8>, &'static str)> {
    let frames = match ext {
        "gif" => GifDecoder::new(Cursor::new(data)).ok().and_then(|d| d.into_frames().take(MAX_FRAMES + 1).collect::<Result<Vec<_>, _>>().ok()),
        "webp" => WebPDecoder::new(Cursor::new(data)).ok().filter(|d| d.has_animation()).and_then(|d| d.into_frames().take(MAX_FRAMES + 1).collect::<Result<Vec<_>, _>>().ok()),
        _ => None,
    };
    if let Some(gif) = frames.and_then(|f| animate(f, size)) {
        return Some((gif, "image/gif"));
    }
    let img: DynamicImage = image::load_from_memory(data).ok()?;
    let small = resize(&img.to_rgba8(), size);
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(small).write_to(&mut Cursor::new(&mut out), ImageFormat::Png).ok()?;
    Some((out, "image/png"))
}

fn reply(bytes: Vec<u8>, mime: &'static str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(bytes))
        .unwrap()
}

pub async fn thumb(State(st): State<AppState>, Path((size, name)): Path<(u32, String)>) -> Result<Response, AppError> {
    let valid = SIZES.contains(&size) && !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') && name.matches('.').count() == 1;
    let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    if !valid || !["png", "jpg", "gif", "webp"].contains(&ext.as_str()) {
        return Err(AppError::NotFound);
    }
    let dir = st.upload_dir.join("thumbs").join(size.to_string());
    for (suffix, mime) in [("gif", "image/gif"), ("png", "image/png")] {
        if let Ok(bytes) = tokio::fs::read(dir.join(format!("{name}.{suffix}"))).await {
            return Ok(reply(bytes, mime));
        }
    }
    let data = tokio::fs::read(st.upload_dir.join(&name)).await.map_err(|_| AppError::NotFound)?;
    let (bytes, mime) = tokio::task::spawn_blocking(move || render(&data, &ext, size))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;
    let suffix = if mime == "image/gif" { "gif" } else { "png" };
    if tokio::fs::create_dir_all(&dir).await.is_ok() {
        let tmp = dir.join(format!("{name}.{suffix}.tmp"));
        if tokio::fs::write(&tmp, &bytes).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, dir.join(format!("{name}.{suffix}"))).await;
        }
    }
    Ok(reply(bytes, mime))
}
