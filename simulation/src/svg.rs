//! Запись кадров симуляции в виде отдельных SVG-файлов.
//!
//! Каждый кадр сохраняется немедленно при вызове [`SvgRecorder::add_frame`].
//! Имена файлов формируются как `<stem>-<XXXX>.svg`, где XXXX — порядковый
//! номер кадра с ведущими нулями.

use crate::gif::FrameTiming;
use crate::unit::viewport::Viewport;
use std::path::PathBuf;

// ── SVG-запись ────────────────────────────────────────────────────────────────

/// Записывает кадры симуляции в отдельные SVG-файлы.
pub(crate) struct SvgRecorder {
    output_dir: PathBuf,
    stem: String,
    frame_count: usize,
}

impl SvgRecorder {
    /// Создаёт рекордер.
    ///
    /// `output_dir` — директория назначения, `stem` — основа имени файла
    /// (обычно имя входного `.lam`-файла без расширения).
    pub(crate) fn new(output_dir: PathBuf, stem: String) -> Self {
        Self {
            output_dir,
            stem,
            frame_count: 0,
        }
    }

    /// Сохраняет очередной кадр как SVG-файл.
    ///
    /// Параметры `w`, `h`, `delay` игнорируются — SVG не растеризуется.
    /// Возвращает [`FrameTiming`] с заполненным только `serial_ms`.
    pub(crate) fn add_frame(
        &mut self,
        viewport: &Viewport,
        _w: u32,
        _h: u32,
        _delay: Option<u16>,
    ) -> Result<FrameTiming, String> {
        self.frame_count += 1;
        let filename = format!("{}-{:04}.svg", self.stem, self.frame_count);
        let path = self.output_dir.join(&filename);

        let t = std::time::Instant::now();
        let svg_str = viewport_to_string(viewport)?;
        let serial_ms = t.elapsed().as_millis();

        std::fs::write(&path, svg_str)
            .map_err(|e| format!("Не удалось записать SVG {}: {e}", path.display()))?;

        Ok(FrameTiming {
            serial_ms,
            parse_ms: 0,
            render_ms: 0,
            quant_ms: 0,
        })
    }

    /// Завершает запись (ничего не делает — кадры уже записаны в `add_frame`).
    pub(crate) fn save(self) -> Result<(), String> {
        Ok(())
    }
}

// ── Вспомогательные функции ───────────────────────────────────────────────────

fn viewport_to_string(viewport: &Viewport) -> Result<String, String> {
    match viewport {
        Viewport::SVG(doc) => Ok(doc.to_string()),
    }
}

// ── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_recorder_save_empty_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let rec = SvgRecorder::new(dir.path().to_path_buf(), "test".into());
        assert!(rec.save().is_ok());
    }

    #[test]
    fn test_svg_recorder_frame_count_increments() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = SvgRecorder::new(dir.path().to_path_buf(), "model".into());
        assert_eq!(rec.frame_count, 0);
        // Создаём минимальный Viewport вручную через SVG-документ
        let doc = svg::Document::new()
            .set("viewBox", (0, 0, 100, 100))
            .set("xmlns", "http://www.w3.org/2000/svg");
        let vp = Viewport::SVG(doc);
        rec.add_frame(&vp, 100, 100, None).unwrap();
        assert_eq!(rec.frame_count, 1);
        let expected_path = dir.path().join("model-0001.svg");
        assert!(expected_path.exists(), "SVG-файл должен быть создан");
    }

    #[test]
    fn test_svg_recorder_naming_format() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = SvgRecorder::new(dir.path().to_path_buf(), "stacker".into());
        let doc = svg::Document::new().set("xmlns", "http://www.w3.org/2000/svg");
        let vp = Viewport::SVG(doc);
        for _ in 0..3 {
            rec.add_frame(&vp, 100, 100, None).unwrap();
        }
        for i in 1..=3u32 {
            let p = dir.path().join(format!("stacker-{i:04}.svg"));
            assert!(p.exists(), "файл {p:?} должен существовать");
        }
    }
}
