use image::{DynamicImage, RgbaImage};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::{path::Path, sync::Mutex};

#[derive(Default)]
pub struct OcrService(Mutex<Option<OcrEngine>>);

impl OcrService {
    pub fn recognize(&self, image: RgbaImage, models: &Path) -> Result<String, String> {
        let mut engine = self
            .0
            .lock()
            .map_err(|_| "Не вдалося отримати OCR. Перезапустіть RustLens.")?;
        if engine.is_none() {
            let load = |name: &str| {
                let path = models.join(name);
                if !path.is_file() {
                    return Err("OCR-моделі відсутні. Виконайте pnpm models:download у папці проєкту та перезапустіть RustLens.".to_string());
                }
                Model::load_file(path)
                    .map_err(|e| format!("Не вдалося прочитати OCR-модель {name}: {e}"))
            };
            *engine = Some(
                OcrEngine::new(OcrEngineParams {
                    detection_model: Some(load("text-detection.onnx")?),
                    recognition_model: Some(load("text-recognition.onnx")?),
                    ..Default::default()
                })
                .map_err(|e| format!("Не вдалося запустити OCR: {e}"))?,
            );
        }
        let image = DynamicImage::ImageRgba8(image);
        // Upscale short text fragments; bound full-screen input to avoid excessive CPU work.
        let image = if image.height() < 80 && image.width() < 1000 {
            image.resize(
                image.width() * 2,
                image.height() * 2,
                image::imageops::FilterType::Triangle,
            )
        } else if image.width() > 2000 || image.height() > 1600 {
            image.resize(2000, 1600, image::imageops::FilterType::Triangle)
        } else {
            image
        };
        let rgb = image.into_rgb8();
        let source =
            ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions()).map_err(|e| e.to_string())?;
        let engine = engine.as_ref().ok_or("OCR недоступний.")?;
        let input = engine.prepare_input(source).map_err(|e| e.to_string())?;
        let text = engine
            .get_text(&input)
            .map_err(|e| format!("Не вдалося розпізнати текст: {e}"))?;
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err(
                "У фрагменті не знайдено тексту. Виділіть текст ближче або введіть його вручну."
                    .into(),
            );
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires downloaded OCR models"]
    fn recognizes_english_fixture() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let image = image::open(root.join("tests/fixtures/english-dialogue.png"))
            .unwrap()
            .into_rgba8();
        let text = OcrService::default()
            .recognize(image, &root.join("models"))
            .unwrap();
        println!("OCR: {text}");
        assert!(
            text.contains("running out of supplies"),
            "Unexpected OCR: {text}"
        );
        assert!(text.contains("northern gate"), "Unexpected OCR: {text}");
    }
}
