//! RGBA-канвас UI поверх пиксельного мира.
//!
//! Мир рисуется в палитровый буфер 640×400 (рейкастер), а весь интерфейс —
//! на отдельный альфа-канвас 1280×800 (2× буфера). В Bevy это второй спрайт
//! над миром: текст/панели не режутся палитрой из 24 цветов и не налезают
//! на экранные текстуры — у канваса есть прозрачность и своя сетка пикселей.

use bevy::ecs::resource::Resource;

/// Размер канваса UI (2× буфера мира 640×400).
pub const CV_W: i32 = 1280;
pub const CV_H: i32 = 800;

#[derive(Resource)]
pub struct Canvas {
    /// RGBA, 4 байта на пиксель.
    pub px: Vec<u8>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            px: vec![0; CV_W as usize * CV_H as usize * 4],
        }
    }

    /// Полная очистка (прозрачный фон).
    pub fn clear(&mut self) {
        self.px.fill(0);
    }

    /// Пиксель с альфа-блендом (src-over). Вне канваса молчит.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: [u8; 3], a: u8) {
        if a == 0 || x < 0 || y < 0 {
            return;
        }
        if x >= CV_W || y >= CV_H {
            return;
        }
        let i = ((y as usize * CV_W as usize) + x as usize) * 4;
        if a == 255 {
            self.px[i] = c[0];
            self.px[i + 1] = c[1];
            self.px[i + 2] = c[2];
            self.px[i + 3] = 255;
            return;
        }
        let f = a as f32 / 255.0;
        let d = &mut self.px[i..i + 4];
        for k in 0..3 {
            let dst = d[k] as f32;
            d[k] = (c[k] as f32 * f + dst * (1.0 - f)).round() as u8;
        }
        let da = d[3] as f32 / 255.0;
        d[3] = ((f + da * (1.0 - f)) * 255.0) as u8;
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3], a: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// Рамка толщиной 1 px по границе прямоугольника.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3], a: u8) {
        for xx in x..x + w {
            self.blend(xx, y, c, a);
            self.blend(xx, y + h - 1, c, a);
        }
        for yy in y..y + h {
            self.blend(x, yy, c, a);
            self.blend(x + w - 1, yy, c, a);
        }
    }

    /// Затмение всего канваса чёрным (для меню паузы).
    pub fn dim(&mut self, a: u8) {
        for y in 0..CV_H {
            for x in 0..CV_W {
                self.blend(x, y, [0, 0, 0], a);
            }
        }
    }

    /// Итоговый кадр: мир 640×400 → ×2, поверх — канвас UI.
    /// Используется и самотестом (BMP-кадры), и живым окном (текстура
    /// экрана) — поэтому итог обязан быть непрозрачным: альфа-байт мира
    /// не гарантирован, а BMP альфу игнорирует.
    pub fn over_world(&self, world_rgba: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; CV_W as usize * CV_H as usize * 4];
        for y in 0..CV_H as usize {
            for x in 0..CV_W as usize {
                let wi = ((y / 2) * 640 + (x / 2)) * 4;
                let oi = (y * CV_W as usize + x) * 4;
                out[oi..oi + 3].copy_from_slice(&world_rgba[wi..wi + 3]);
                out[oi + 3] = 255;
            }
        }
        // альфа-наложение канваса UI
        for i in 0..out.len() / 4 {
            let a = self.px[i * 4 + 3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            for k in 0..3 {
                let v = self.px[i * 4 + k] as f32 * a + out[i * 4 + k] as f32 * (1.0 - a);
                out[i * 4 + k] = v as u8;
            }
        }
        out
    }
}
