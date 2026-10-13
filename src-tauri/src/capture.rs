use crate::geometry::Rect;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::Serialize;
use std::io::Cursor;

pub struct Frame {
    pub id: u64,
    pub origin: (i32, i32),
    pub image: RgbaImage,
    pub preview: FramePreview,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePreview {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub image_url: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionPreview {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub image_url: String,
}

fn png_url(image: &RgbaImage) -> Result<String, String> {
    let mut buffer = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| format!("Не вдалося підготувати зображення: {e}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(buffer.into_inner())
    ))
}

pub fn capture(id: u64, cursor: (i32, i32)) -> Result<Frame, String> {
    let monitor = xcap::Monitor::from_point(cursor.0, cursor.1)
        .map_err(|e| format!("Не вдалося знайти монітор під курсором: {e}"))?;
    let origin = (
        monitor.x().map_err(|e| e.to_string())?,
        monitor.y().map_err(|e| e.to_string())?,
    );
    let image = monitor
        .capture_image()
        .map_err(|e| format!("Не вдалося захопити екран. Спробуйте віконний режим гри. {e}"))?;
    let preview = FramePreview {
        id,
        width: image.width(),
        height: image.height(),
        image_url: png_url(&image)?,
    };
    Ok(Frame {
        id,
        origin,
        image,
        preview,
    })
}

impl Frame {
    pub fn crop(&self, rect: Rect) -> Result<SelectionPreview, String> {
        let rect = rect.validate(self.image.width(), self.image.height())?;
        let image = image::imageops::crop_imm(&self.image, rect.x, rect.y, rect.width, rect.height)
            .to_image();
        Ok(SelectionPreview {
            id: self.id,
            width: rect.width,
            height: rect.height,
            image_url: png_url(&image)?,
        })
    }
}
