//! Процедурные спрайты NPC (32x64) и иконки посуды (16x16), индексы палитры.
//! 255 = прозрачный пиксель.

use crate::palette::idx;

pub const TRANSPARENT: u8 = 255;
pub const SPR_W: usize = 32;
pub const SPR_H: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NpcKind {
    Grandma,
    Worker,
    Mama,
    Student,
    Militsioner,
    Hooligan,
    Sanepid,
}

impl NpcKind {
    pub fn name(self) -> &'static str {
        match self {
            NpcKind::Grandma => "БАБУШКА",
            NpcKind::Worker => "РАБОЧИЙ",
            NpcKind::Mama => "МАМА",
            NpcKind::Student => "СТУДЕНТ",
            NpcKind::Militsioner => "МИЛИЦИОНЕР",
            NpcKind::Hooligan => "ХУЛИГАН",
            NpcKind::Sanepid => "САНЭПИД",
        }
    }
}

pub struct SpriteSheet {
    /// npc[kind][y*32+x]
    pub npc: Vec<Vec<u8>>,
}

/// Пиксель в буфере шириной `w` (границы — по фактической длине буфера).
fn put(s: &mut [u8], w: usize, x: i32, y: i32, c: u8) {
    if x >= 0 && y >= 0 && (x as usize) < w {
        let i = y as usize * w + x as usize;
        if i < s.len() {
            s[i] = c;
        }
    }
}

/// Заливает прямоугольник с чёрной обводкой по периметру.
fn block(s: &mut [u8], w: usize, x0: i32, y0: i32, rw: i32, h: i32, c: u8) {
    for y in y0..y0 + h {
        for x in x0..x0 + rw {
            let border = x == x0 || y == y0 || x == x0 + rw - 1 || y == y0 + h - 1;
            put(s, w, x, y, if border { idx::BLACK } else { c });
        }
    }
}

impl SpriteSheet {
    pub fn generate() -> Self {
        let npc = vec![
            person(NpcKind::Grandma),
            person(NpcKind::Worker),
            person(NpcKind::Mama),
            person(NpcKind::Student),
            person(NpcKind::Militsioner),
            person(NpcKind::Hooligan),
            person(NpcKind::Sanepid),
        ];
        Self { npc }
    }

    pub fn npc_sprite(&self, k: NpcKind) -> &[u8] {
        &self.npc[k as usize]
    }
}

/// Моделка 32x64 с уникальным образом для каждого типа.
fn person(kind: NpcKind) -> Vec<u8> {
    let mut s = vec![TRANSPARENT; SPR_W * SPR_H];
    let w = SPR_W;
    let skin = idx::SKIN;

    let (top, pants) = match kind {
        NpcKind::Grandma => (idx::CEILING, idx::POLICE),   // серая кофта, синяя юбка
        NpcKind::Worker => (idx::TILE_DARK, idx::CEILING), // спецовка
        NpcKind::Mama => (idx::GREEN, idx::GREEN),         // платье зелёное
        NpcKind::Student => (idx::POLICE, idx::TILE_DARK), // куртка синяя
        NpcKind::Militsioner => (idx::POLICE, idx::POLICE),
        NpcKind::Hooligan => (idx::BLACK, idx::CEILING),   // кожанка
        NpcKind::Sanepid => (idx::WHITE, idx::METAL_DARK), // халат
    };

    // ноги и ботинки
    block(&mut s, w, 9, 39, 6, 20, pants);
    block(&mut s, w, 17, 39, 6, 20, pants);
    block(&mut s, w, 8, 59, 8, 4, idx::BLACK);
    block(&mut s, w, 16, 59, 8, 4, idx::BLACK);

    // юбки поверх ног
    match kind {
        NpcKind::Grandma => block(&mut s, w, 7, 31, 18, 27, idx::POLICE),
        NpcKind::Mama => block(&mut s, w, 7, 31, 18, 26, idx::GREEN),
        _ => {}
    }

    // торс
    block(&mut s, w, 8, 17, 16, 22, top);

    // детали одежды
    match kind {
        NpcKind::Worker => {
            block(&mut s, w, 8, 28, 16, 3, idx::METAL_DARK); // ремень
            put(&mut s, w, 16, 29, idx::LAMP); // пряжка
            for x in 13..19 {
                put(&mut s, w, x, 18, idx::PLASTER); // воротник
            }
        }
        NpcKind::Militsioner => {
            block(&mut s, w, 8, 17, 4, 3, idx::METAL); // погоны
            block(&mut s, w, 20, 17, 4, 3, idx::METAL);
            block(&mut s, w, 8, 28, 16, 3, idx::METAL_DARK);
            put(&mut s, w, 16, 29, idx::LAMP);
            for y in (21..28).step_by(3) {
                put(&mut s, w, 16, y, idx::LAMP); // пуговицы
            }
        }
        NpcKind::Hooligan => {
            for x in 11..21 {
                put(&mut s, w, x, 18, idx::SHADOW); // широкий воротник
            }
            for y in 19..38 {
                put(&mut s, w, 16, y, idx::METAL); // молния
            }
        }
        NpcKind::Sanepid => {
            for y in (20..38).step_by(4) {
                put(&mut s, w, 16, y, idx::METAL_DARK); // пуговицы халата
            }
            block(&mut s, w, 10, 29, 5, 4, idx::METAL); // карман
        }
        NpcKind::Student => {
            for x in 13..19 {
                put(&mut s, w, x, 17, idx::WHITE); // воротник рубашки
            }
            block(&mut s, w, 2, 31, 6, 8, idx::PLASTER); // книга в руке
            for y in 32..39 {
                put(&mut s, w, 3, y, idx::RED_DARK); // корешок
            }
        }
        NpcKind::Mama => {
            block(&mut s, w, 8, 27, 16, 3, idx::PLASTER); // пояс
            block(&mut s, w, 2, 35, 7, 6, idx::RED); // сумочка
        }
        NpcKind::Grandma => {
            block(&mut s, w, 25, 35, 6, 7, idx::WOOD_DARK); // авоська
        }
    }

    // руки и кисти
    block(&mut s, w, 4, 18, 4, 16, top);
    block(&mut s, w, 24, 18, 4, 16, top);
    block(&mut s, w, 4, 34, 4, 4, skin);
    block(&mut s, w, 24, 34, 4, 4, skin);

    // шаль поверх плеч
    if kind == NpcKind::Grandma {
        block(&mut s, w, 6, 16, 20, 7, idx::RED);
    }

    // шея и голова
    block(&mut s, w, 13, 15, 6, 2, skin);
    block(&mut s, w, 10, 4, 12, 12, skin);
    put(&mut s, w, 13, 9, idx::BLACK);
    put(&mut s, w, 18, 9, idx::BLACK);
    for x in 14..19 {
        put(&mut s, w, x, 13, idx::BLACK);
    }

    // головные уборы и волосы
    match kind {
        NpcKind::Grandma => {
            // красный платок: тулья и бока вдоль лица
            for x in 9..23 {
                for y in 2..8 {
                    let edge = y == 2 || x == 9 || x == 22;
                    put(&mut s, w, x, y, if edge { idx::BLACK } else { idx::RED });
                }
            }
            for y in 8..15 {
                put(&mut s, w, 9, y, idx::RED);
                put(&mut s, w, 10, y, idx::RED_DARK);
                put(&mut s, w, 21, y, idx::RED_DARK);
                put(&mut s, w, 22, y, idx::RED);
            }
        }
        NpcKind::Mama => {
            // шляпка с цветком
            block(&mut s, w, 10, 1, 12, 3, idx::PLASTER);
            block(&mut s, w, 7, 4, 18, 2, idx::PLASTER);
            put(&mut s, w, 12, 2, idx::RED);
            put(&mut s, w, 13, 2, idx::RED_DARK);
        }
        NpcKind::Worker => {
            block(&mut s, w, 9, 1, 14, 4, idx::POLICE); // кепка
            block(&mut s, w, 8, 5, 16, 2, idx::POLICE); // козырёк
            for x in 10..22 {
                put(&mut s, w, x, 7, idx::WOOD_DARK); // волосы
            }
        }
        NpcKind::Student => {
            for x in 9..23 {
                for y in 2..6 {
                    let edge = y == 2 || x == 9 || x == 22;
                    put(&mut s, w, x, y, if edge { idx::BLACK } else { idx::WOOD_DARK });
                }
            }
        }
        NpcKind::Militsioner => {
            block(&mut s, w, 9, 1, 14, 4, idx::BLACK); // фуражка
            block(&mut s, w, 8, 5, 16, 2, idx::BLACK); // козырёк
            for x in 9..23 {
                put(&mut s, w, x, 4, idx::RED_DARK); // околыш
            }
            put(&mut s, w, 16, 2, idx::LAMP); // кокарда
        }
        NpcKind::Hooligan => {
            // осёлка
            for x in 9..23 {
                for y in 1..6 {
                    if (x + y) % 3 != 0 {
                        put(&mut s, w, x, y, idx::BLACK);
                    }
                }
            }
        }
        NpcKind::Sanepid => {
            block(&mut s, w, 9, 1, 14, 4, idx::WHITE); // шапочка
            block(&mut s, w, 8, 5, 16, 2, idx::WHITE);
        }
    }
    s
}

/// Иконка посуды 16x16 для HUD; filled — с квасом.
pub fn vessel_icon(kind: Vessel, filled: bool) -> Vec<u8> {
    let mut s = vec![TRANSPARENT; 16 * 16];
    let w = 16;
    match kind {
        Vessel::Glass => {
            block(&mut s, w, 3, 2, 10, 13, if filled { idx::KVASS_MID } else { idx::METAL });
            if filled {
                for y in 3..6 {
                    for x in 4..12 {
                        put(&mut s, w, x, y, idx::KVASS_LIGHT);
                    }
                }
            }
            // граны
            put(&mut s, w, 6, 6, if filled { idx::KVASS_DARK } else { idx::METAL_DARK });
            put(&mut s, w, 10, 6, if filled { idx::KVASS_DARK } else { idx::METAL_DARK });
        }
        Vessel::Jar => {
            block(&mut s, w, 3, 3, 10, 12, if filled { idx::KVASS_MID } else { idx::METAL });
            // крышка
            for x in 2..14 {
                put(&mut s, w, x, 1, idx::METAL);
                put(&mut s, w, x, 2, idx::METAL_DARK);
            }
            if filled {
                for y in 4..7 {
                    for x in 4..12 {
                        put(&mut s, w, x, y, idx::KVASS_LIGHT);
                    }
                }
            }
        }
        Vessel::Bottle => {
            // горлышко
            block(&mut s, w, 6, 1, 4, 5, idx::BOTTLE_GLASS);
            // тело
            block(&mut s, w, 4, 6, 8, 9, if filled { idx::KVASS_DARK } else { idx::BOTTLE_GLASS });
            // этикетка
            for y in 9..12 {
                for x in 5..11 {
                    put(&mut s, w, x, y, idx::PLASTER);
                }
            }
        }
    }
    s
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vessel {
    Glass,
    Jar,
    Bottle,
}

impl Vessel {
    pub fn price(self) -> i32 {
        match self {
            Vessel::Glass => 5,
            Vessel::Jar => 10,
            Vessel::Bottle => 15,
        }
    }
    /// Сколько порций кваса из бочки.
    pub fn portions(self) -> u32 {
        match self {
            Vessel::Glass => 1,
            Vessel::Jar => 2,
            Vessel::Bottle => 4,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Vessel::Glass => "СТАКАН",
            Vessel::Jar => "БАНКА",
            Vessel::Bottle => "БУТЫЛКА",
        }
    }
}
