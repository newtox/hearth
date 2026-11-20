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

pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub out_w: u32,
    pub out_h: u32,
}

impl Crop {
    pub fn parse(rect: &str, out: &str) -> Option<Crop> {
        let n: Vec<u32> = rect.split(',').map(|v| v.trim().parse().ok()).collect::<Option<_>>()?;
        let o: Vec<u32> = out.split('x').map(|v| v.trim().parse().ok()).collect::<Option<_>>()?;
        let ([x, y, w, h], [out_w, out_h]) = (n.as_slice(), o.as_slice()) else {
            return None;
        };
        let ok = *w > 0 && *h > 0 && (16..=1200).contains(out_w) && (16..=1200).contains(out_h);
        ok.then_some(Crop { x: *x, y: *y, w: *w, h: *h, out_w: *out_w, out_h: *out_h })
    }

    fn apply(&self, img: &RgbaImage) -> Option<RgbaImage> {
        let (iw, ih) = img.dimensions();
        if self.x >= iw || self.y >= ih {
            return None;
        }
        let w = self.w.min(iw - self.x);
        let h = self.h.min(ih - self.y);
        let part = image::imageops::crop_imm(img, self.x, self.y, w, h).to_image();
        let scale = (self.out_w as f64 / w as f64).min(self.out_h as f64 / h as f64).min(1.0);
        let (tw, th) = (((w as f64) * scale).round().max(1.0) as u32, ((h as f64) * scale).round().max(1.0) as u32);
        Some(if (tw, th) == (w, h) { part } else { image::imageops::resize(&part, tw, th, FilterType::Lanczos3) })
    }
}

pub fn crop_gif(data: &[u8], crop: &Crop) -> Option<Vec<u8>> {
    let frames = GifDecoder::new(Cursor::new(data)).ok()?.into_frames().take(MAX_FRAMES + 1).collect::<Result<Vec<_>, _>>().ok()?;
    if frames.is_empty() || frames.len() > MAX_FRAMES {
        return None;
    }
    let cut = frames.into_iter().map(|f| Some(Frame::from_parts(crop.apply(f.buffer())?, 0, 0, f.delay()))).collect::<Option<Vec<_>>>()?;
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.set_repeat(Repeat::Infinite).ok()?;
        enc.encode_frames(cut).ok()?;
    }
    Some(out)
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
