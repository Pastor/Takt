use crate::unit::viewport::Viewport;
use std::path::{Path, PathBuf};

// ── GIF-запись ────────────────────────────────────────────────────────────────

/// Записывает кадры симуляции в анимированный GIF-файл.
///
/// Каждый кадр добавляется как RGBA-изображение, растеризованное из SVG-документа
/// через resvg. Для итогового GIF применяется квантизация цвета до 256 оттенков.
pub(crate) struct GifRecorder {
    frames: Vec<RgbaFrame>,
    output_path: PathBuf,
    frame_delay: u16,
}

struct RgbaFrame {
    width: u16,
    height: u16,
    data: Vec<u8>,
}

impl GifRecorder {
    /// Создаёт новый рекордер.
    ///
    /// `frame_delay` — задержка между кадрами в единицах 1/100 секунды.
    pub(crate) fn new(output_path: &Path, frame_delay: u16) -> Self {
        Self {
            frames: Vec::new(),
            output_path: output_path.to_path_buf(),
            frame_delay,
        }
    }

    /// Добавляет кадр из SVG-документа Viewport.
    pub(crate) fn add_frame(
        &mut self,
        viewport: &Viewport,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let svg_str = viewport_to_string(viewport)?;
        let pixmap = render_svg(&svg_str, width, height)?;
        self.frames.push(RgbaFrame {
            width: width as u16,
            height: height as u16,
            data: pixmap,
        });
        Ok(())
    }

    /// Сохраняет все накопленные кадры в GIF-файл.
    pub(crate) fn save(self) -> Result<(), String> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let file = std::fs::File::create(&self.output_path).map_err(|e| {
            format!(
                "Не удалось создать GIF {}: {}",
                self.output_path.display(),
                e
            )
        })?;
        let w = self.frames[0].width;
        let h = self.frames[0].height;
        let mut encoder = gif::Encoder::new(file, w, h, &[])
            .map_err(|e| format!("Ошибка создания GIF encoder: {e}"))?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| format!("Ошибка настройки GIF repeat: {e}"))?;

        for frame_data in self.frames {
            let frame = rgba_to_gif_frame(
                &frame_data.data,
                frame_data.width,
                frame_data.height,
                self.frame_delay,
            );
            encoder
                .write_frame(&frame)
                .map_err(|e| format!("Ошибка записи кадра GIF: {e}"))?;
        }
        Ok(())
    }
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn viewport_to_string(viewport: &Viewport) -> Result<String, String> {
    match viewport {
        Viewport::SVG(doc) => Ok(doc.to_string()),
    }
}

fn render_svg(svg_str: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_str, &options)
        .map_err(|e| format!("Ошибка парсинга SVG: {e}"))?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "Не удалось создать Pixmap".to_string())?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);

    let scale_x = width as f32 / tree.size().width();
    let scale_y = height as f32 / tree.size().height();
    let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap.data().to_vec())
}

fn rgba_to_gif_frame(rgba: &[u8], width: u16, height: u16, delay: u16) -> gif::Frame<'static> {
    // Квантизация: для каждого пикселя берём только R,G,B (пропускаем A)
    // и собираем RGB-вектор для встроенной квантизации gif::Frame.
    let pixel_count = (width as usize) * (height as usize);
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        let base = i * 4;
        if base + 2 < rgba.len() {
            rgb.push(rgba[base]); // R
            rgb.push(rgba[base + 1]); // G
            rgb.push(rgba[base + 2]); // B
        }
    }
    let mut frame = gif::Frame::from_rgb(width, height, &rgb);
    frame.delay = delay;
    frame
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gif_recorder_save_empty_is_ok() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let recorder = GifRecorder::new(tmp.path(), 50);
        assert!(recorder.save().is_ok());
    }

    #[test]
    fn test_render_svg_produces_rgba() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="red"/></svg>"#;
        let result = render_svg(svg, 10, 10);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 10 * 10 * 4);
    }

    #[test]
    fn test_rgba_to_gif_frame_has_delay() {
        let rgba = vec![255u8; 4 * 4 * 4]; // 4x4 белых пикселей
        let frame = rgba_to_gif_frame(&rgba, 4, 4, 42);
        assert_eq!(frame.delay, 42);
    }
}
