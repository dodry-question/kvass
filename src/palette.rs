//! Палитра 24 цветов (строго по ТЗ §2.2) + Doom-colormap, дизеринг Байера 4x4.

/// 24 цвета палитры, индексы 0..23.
pub const PALETTE: [[u8; 3]; 24] = [
    [0xE8, 0xB8, 0x4B], // 0  Квас светлый (янтарь, пена)
    [0xB8, 0x73, 0x33], // 1  Квас основной (медь/мёд)
    [0x6B, 0x3A, 0x14], // 2  Квас тёмный (гуща)
    [0xC8, 0x9F, 0x6B], // 3  Дерево светлое
    [0x7A, 0x52, 0x30], // 4  Дерево тёмное
    [0x9A, 0xA8, 0x9A], // 5  Кафель
    [0x5C, 0x6B, 0x5C], // 6  Кафель тень
    [0xC9, 0xC0, 0xA8], // 7  Штукатурка
    [0x3B, 0x31, 0x28], // 8  Потолок
    [0x4A, 0x40, 0x38], // 9  Пол (линолеум)
    [0xC2, 0x2E, 0x2E], // 10 Красный (СССР)
    [0x7A, 0x1A, 0x1A], // 11 Красный тёмный
    [0x2E, 0x6B, 0x3A], // 12 Зелёный флаг
    [0x8A, 0x8F, 0x94], // 13 Металл
    [0x4A, 0x4E, 0x52], // 14 Металл тень
    [0xD8, 0xA8, 0x7C], // 15 Кожа/лица
    [0xE8, 0xE4, 0xD8], // 16 Белый
    [0x18, 0x14, 0x10], // 17 Чёрный
    [0x24, 0x1E, 0x16], // 18 Тень глубокая
    [0x2E, 0x4A, 0x7A], // 19 Синий милиция
    [0xFF, 0xE8, 0xA0], // 20 Лампа
    [0x87, 0xA8, 0xC9], // 20 Небо
    [0x4A, 0x7A, 0x3A], // 22 Зелень снаружи
    [0x5A, 0x7A, 0x5A], // 23 Стекло бутылки
];

/// Индексы палитры для частых случаев.
pub mod idx {
    pub const KVASS_LIGHT: u8 = 0;
    pub const KVASS_MID: u8 = 1;
    pub const KVASS_DARK: u8 = 2;
    pub const WOOD_LIGHT: u8 = 3;
    pub const WOOD_DARK: u8 = 4;
    pub const TILE: u8 = 5;
    pub const TILE_DARK: u8 = 6;
    pub const PLASTER: u8 = 7;
    pub const CEILING: u8 = 8;
    pub const FLOOR: u8 = 9;
    pub const RED: u8 = 10;
    pub const RED_DARK: u8 = 11;
    pub const GREEN: u8 = 12;
    pub const METAL: u8 = 13;
    pub const METAL_DARK: u8 = 14;
    pub const SKIN: u8 = 15;
    pub const WHITE: u8 = 16;
    pub const BLACK: u8 = 17;
    pub const SHADOW: u8 = 18;
    pub const POLICE: u8 = 19;
    pub const LAMP: u8 = 20;
    pub const SKY: u8 = 21;
    pub const LEAVES: u8 = 22;
    pub const BOTTLE_GLASS: u8 = 23;
}

pub const NUM_LIGHTS: usize = 16;

/// Матрица Байера 4x4 (значения 0..16).
pub const BAYER4: [[u8; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

/// Doom-colormap: [цвет палитры][ступень яркости 0..15] -> цвет палитры.
pub fn build_colormap() -> [[u8; NUM_LIGHTS]; 24] {
    let mut cm = [[0u8; NUM_LIGHTS]; 24];
    for (ci, color) in PALETTE.iter().enumerate() {
        for lvl in 0..NUM_LIGHTS {
            let f = lvl as f32 / (NUM_LIGHTS - 1) as f32;
            let target = [
                (color[0] as f32 * f) as i32,
                (color[1] as f32 * f) as i32,
                (color[2] as f32 * f) as i32,
            ];
            let mut best = 0usize;
            let mut best_d = i32::MAX;
            for (bi, c2) in PALETTE.iter().enumerate() {
                let dr = target[0] - c2[0] as i32;
                let dg = target[1] - c2[1] as i32;
                let db = target[2] - c2[2] as i32;
                let d = dr * dr + dg * dg + db * db;
                if d < best_d {
                    best_d = d;
                    best = bi;
                }
            }
            cm[ci][lvl] = best as u8;
        }
    }
    cm
}

/// Ступень яркости 0..15 с дизерингом Байера.
/// `bright` — 0.0 (темно) .. 1.0 (ярко), `bayer` — 0..15.
#[inline]
pub fn light_band(bright: f32, bayer: u8) -> usize {
    let v = bright * (NUM_LIGHTS as f32 - 1.0) + (bayer as f32 / 16.0) - 0.5;
    let v = v.floor();
    (v.max(0.0) as usize).min(NUM_LIGHTS - 1)
}
