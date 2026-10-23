//! Процедурные спрайты NPC (32x64) и иконки посуды (16x16), индексы палитры.
//! 255 = прозрачный пиксель.

use bevy::prelude::{Component, Vec2};

use crate::palette::idx;

pub const TRANSPARENT: u8 = 255;
pub const SPR_W: usize = 32;
pub const SPR_H: usize = 64;

/// Рост NPC относительно полной стены-тайла (1.0 = стена до потолка).
pub const NPC_SCALE: f32 = 0.75;
/// Рост урны (ТЗ №5): 0.5 стены ≈ 1.0 м — «по пояс» человеку (NPC 0.75 ≈ 1.75 м).
pub const TRASH_SCALE: f32 = 0.5;
/// На каком расстоянии от статичного предмета срабатывает E
/// (игрок подошёл вплотную; тайловые станции в приоритете — см. interact).
pub const PROP_REACH: f32 = 1.3;

/// Статичные предметы интерьера — билборд-спрайты (не NPC, не тайлы).
/// ТЗ №5: мусорка должна стоять у стены как предмет, а не стеной-кубом.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropKind {
    /// Советская металлическая урна на ножках.
    TrashBin,
}

impl PropKind {
    pub fn name(self) -> &'static str {
        match self {
            PropKind::TrashBin => "МУСОРКА",
        }
    }
    /// Рост относительно стены (см. TRASH_SCALE).
    pub fn scale(self) -> f32 {
        match self {
            PropKind::TrashBin => TRASH_SCALE,
        }
    }
    /// Подсказка «что перед вами», если игрок подошёл близко.
    pub fn hint(self) -> Option<(&'static str, &'static str)> {
        match self {
            PropKind::TrashBin => Some(("E", "МУСОРКА - ВЫБРОСИТЬ ПРЕДМЕТ ИЗ РУК")),
        }
    }
}

/// Сущность статичного предмета: спавнится в setup(), рисуется той же
/// billboard-системой, что и покупатели (`Renderer::draw_props`).
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct WorldProp {
    pub kind: PropKind,
    /// Координаты на полу (центр «ножек» спрайта).
    pub pos: Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NpcKind {
    Grandma,
    Worker,
    Mama,
    Student,
    Militsioner,
    Hooligan,
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
        }
    }
}

pub struct SpriteSheet {
    /// npc[kind][y*32+x]
    pub npc: Vec<Vec<u8>>,
    /// prop[kind][y*32+x] — статичные предметы интерьера
    pub props: Vec<Vec<u8>>,
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
        ];
        let props = vec![trash_bin()];
        Self { npc, props }
    }

    pub fn npc_sprite(&self, k: NpcKind) -> &[u8] {
        &self.npc[k as usize]
    }

    pub fn prop_sprite(&self, k: crate::sprites::PropKind) -> &[u8] {
        &self.props[k as usize]
    }
}

/// Урна-мусорка (ТЗ №5): металлическая бадья на ножках, широкая крышка-обод,
/// торчат скомканные бумажки. Нижняя строка буфера = пол (спрайт стоит на
/// полу), рост задаётся масштабом `PropKind::scale` — «по пояс», не монумент.
fn trash_bin() -> Vec<u8> {
    let mut s = vec![TRANSPARENT; SPR_W * SPR_H];
    let w = SPR_W;
    let body = idx::METAL_DARK; // тёмный корпус — контрастирует со светлым кафелем
    let dark = idx::SHADOW;

    // скомканный мусор поверх крышки — блобы, а не сетка
    for (x, y, c) in [
        (10, 11, idx::PLASTER),
        (11, 12, idx::WHITE),
        (12, 11, idx::PLASTER),
        (16, 10, idx::WHITE),
        (17, 11, idx::PLASTER),
        (20, 12, idx::RED_DARK), // обёртка
        (21, 11, idx::PLASTER),
        (24, 12, idx::GREEN), // огрызок
    ] {
        put(&mut s, w, x, y, c);
        put(&mut s, w, x, y + 1, c);
    }

    // крышка: купол и широкий обод (самая широкая часть урны)
    for x in 7..25 {
        put(&mut s, w, x, 13, idx::BLACK);
        put(&mut s, w, x, 14, body);
        put(&mut s, w, x, 15, idx::METAL);
        put(&mut s, w, x, 16, idx::PLASTER); // блик по куполу
        put(&mut s, w, x, 17, body);
    }
    block(&mut s, w, 2, 18, 28, 4, idx::METAL); // обод — светлый металл
    for x in 3..29 {
        put(&mut s, w, x, 19, idx::PLASTER); // светлая кромка обода
    }

    // тело: сужается книзу, объём (тёмные края + блик) и точечная грязь
    for y in 22..57 {
        let inset = if y >= 50 {
            3
        } else if y >= 40 {
            2
        } else if y >= 30 {
            1
        } else {
            0
        };
        let x0 = 4 + inset;
        let x1 = 28 - inset;
        let width = x1 - x0;
        let mid = width / 2;
        for x in x0..x1 {
            let rel = x - x0;
            // мультипликативный хэш — рассыпанные точки, НЕ диагональные линии
            let h = (x as u32).wrapping_mul(2_654_435_761) ^ (y as u32).wrapping_mul(40_503);
            let c = if rel == 0 || rel == width - 1 {
                idx::BLACK // контур
            } else if rel < 3 || rel >= width - 3 {
                dark // тень по краям — объём цилиндра
            } else if rel == mid || rel == mid - 1 {
                if h & 7 == 0 {
                    body
                } else {
                    idx::METAL // вертикальный блик с мелкими «царапинами»
                }
            } else if h % 37 == 0 {
                idx::LEAVES // зеленоватая грязь («грязно-зелёная»)
            } else if h % 23 == 0 {
                idx::METAL // потёртости
            } else {
                body
            };
            put(&mut s, w, x, y, c);
        }
        // рёбра жёсткости — светлые линии на тёмном корпусе
        if y == 30 || y == 38 || y == 46 {
            for x in (x0 + 2)..(x1 - 2) {
                put(&mut s, w, x, y, idx::METAL);
            }
        }
    }

    // дно и три ножки (спрайт достаёт до нижней строки буфера — стоит на полу)
    block(&mut s, w, 5, 57, 22, 3, idx::METAL_DARK);
    for &lx in [6, 14, 22].iter() {
        block(&mut s, w, lx, 60, 4, 4, idx::METAL_DARK);
    }
    s
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

    // объём: дизеринг-тень на правом боку торса и на левой ноге
    for y in 18..38 {
        if y % 2 == 0 {
            put(&mut s, w, 22, y, idx::SHADOW);
        }
    }
    for y in 40..58 {
        put(&mut s, w, 9, y, idx::SHADOW);
    }

    // морщинистое лицо: глаза-точки шире, нос
    put(&mut s, w, 12, 9, idx::BLACK);
    put(&mut s, w, 19, 9, idx::BLACK);
    put(&mut s, w, 15, 11, idx::RED_DARK); // нос
    put(&mut s, w, 11, 11, idx::RED_DARK); // морщинки у глаз
    put(&mut s, w, 20, 11, idx::RED_DARK);

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
            // фартук поверх юбки
            for y in 33..47 {
                for x in 11..21 {
                    let c = if x == 20 || (x + y) % 5 == 0 {
                        idx::TILE_DARK
                    } else {
                        idx::PLASTER
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // узелок платка на груди
            put(&mut s, w, 12, 23, idx::RED_DARK);
            put(&mut s, w, 13, 23, idx::RED_DARK);
            put(&mut s, w, 12, 24, idx::RED_DARK);
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
    }
    s
}

/// Иконка посуды 16x16 для HUD; filled — с квасом.
pub fn vessel_icon(kind: Vessel, filled: bool) -> Vec<u8> {
    let mut s = vec![TRANSPARENT; 16 * 16];
    let w = 16;
    match kind {
        Vessel::Glass => {
            // гранёный стакан: сплошное стекло/квас, две тонкие грани, блик
            for y in 2..15 {
                let inset = if y > 12 { 1 } else { 0 };
                for x in (3 + inset)..(13 - inset) {
                    let c = if filled {
                        if y <= 4 {
                            idx::KVASS_LIGHT
                        } else {
                            idx::KVASS_MID
                        }
                    } else {
                        idx::TILE
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // грани — тонкие тёмные линии (чтобы читался «гранёный»)
            for y in 5..14 {
                put(
                    &mut s,
                    w,
                    6,
                    y,
                    if filled { idx::KVASS_DARK } else { idx::TILE_DARK },
                );
                put(
                    &mut s,
                    w,
                    10,
                    y,
                    if filled { idx::KVASS_DARK } else { idx::TILE_DARK },
                );
            }
            // блик слева
            for y in 5..13 {
                put(
                    &mut s,
                    w,
                    4,
                    y,
                    if filled { idx::KVASS_LIGHT } else { idx::WHITE },
                );
            }
            // ободок и дно
            for x in 3..13 {
                put(
                    &mut s,
                    w,
                    x,
                    2,
                    if filled { idx::KVASS_LIGHT } else { idx::WHITE },
                );
                put(&mut s, w, x, 15, idx::TILE_DARK);
            }
        }
        Vessel::Jar => {
            // пузатая банка: узкое горлышко с рельефом, широкое тело
            block(&mut s, w, 5, 1, 6, 3, idx::TILE); // горлышко
            put(&mut s, w, 6, 2, idx::WHITE); // рельеф-блик
            put(&mut s, w, 9, 2, idx::TILE_DARK);
            for y in 4..15 {
                let (x0, x1) = match y {
                    4 => (4, 12),
                    5 => (3, 13),
                    6..=12 => (2, 14),
                    _ => (3, 13),
                };
                for x in x0..x1 {
                    let c = if filled && y >= 5 {
                        idx::KVASS_MID
                    } else {
                        idx::TILE
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // крышка металлическая
            for x in 4..12 {
                put(&mut s, w, x, 0, idx::METAL);
            }
            if filled {
                for y in 5..8 {
                    for x in 4..12 {
                        put(&mut s, w, x, y, idx::KVASS_LIGHT);
                    }
                }
                put(&mut s, w, 3, 8, idx::KVASS_DARK);
                put(&mut s, w, 12, 9, idx::KVASS_DARK);
            } else {
                for (x, y) in [(4, 8), (11, 10), (5, 12)] {
                    put(&mut s, w, x, y, idx::WHITE);
                }
            }
        }
        Vessel::Bottle => {
            // пробка
            block(&mut s, w, 6, 0, 4, 3, idx::WOOD_LIGHT);
            // горлышко
            block(&mut s, w, 6, 3, 4, 3, idx::BOTTLE_GLASS);
            // тело (плечи расширяются)
            for y in 6..15 {
                let (x0, x1) = match y {
                    6 => (5, 11),
                    7 => (4, 12),
                    _ => (4, 12),
                };
                for x in x0..x1 {
                    let c = if filled {
                        idx::KVASS_DARK
                    } else {
                        idx::BOTTLE_GLASS
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // блик стекла
            put(&mut s, w, 5, 8, idx::TILE);
            put(&mut s, w, 5, 9, idx::TILE);
            // этикетка
            for y in 9..13 {
                for x in 6..11 {
                    put(&mut s, w, x, y, idx::PLASTER);
                }
            }
            for x in 6..11 {
                put(&mut s, w, x, 10, if filled { idx::RED } else { idx::RED_DARK });
            }
        }
    }
    s
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropIcon {
    /// Горсть сухарей: мешковина с кусочками хлеба.
    Croutons,
    /// Сахар с изюмом: бумажный пакет.
    Sugar,
    /// Ведро воды: металлическое с дужкой.
    Bucket,
}

/// Иконки предметов 16×16 (склад, слоты, руки). 255 = прозрачный.
pub fn prop_icon(k: PropIcon) -> Vec<u8> {
    let mut s = vec![TRANSPARENT; 16 * 16];
    let w = 16;
    let wood = idx::WOOD_LIGHT;
    let wood_d = idx::WOOD_DARK;
    match k {
        PropIcon::Croutons => {
            // кусочки бородинского в горлышке
            for (x, y, c) in [
                (5, 2, idx::KVASS_DARK),
                (7, 1, idx::KVASS_MID),
                (9, 2, idx::KVASS_DARK),
                (6, 3, idx::KVASS_DARK),
                (8, 3, idx::KVASS_MID),
                (10, 3, idx::KVASS_MID),
            ] {
                put(&mut s, w, x, y, c);
            }
            // завязка горлышка
            block(&mut s, w, 4, 4, 8, 3, idx::WOOD_DARK);
            // корпус мешка с плетением
            for y in 7..15 {
                let (x0, x1) = if y == 7 || y == 14 {
                    (3, 13)
                } else {
                    (2, 14)
                };
                for x in x0..x1 {
                    let weave = if (x + y) % 2 == 0 { wood } else { wood_d };
                    put(&mut s, w, x, y, weave);
                }
                // чёрная окантовка силуэта
                put(&mut s, w, x0 - 1, y, idx::BLACK);
                put(&mut s, w, x1, y, idx::BLACK);
            }
            // складка-блик
            put(&mut s, w, 4, 8, idx::PLASTER);
            put(&mut s, w, 4, 9, wood);
        }
        PropIcon::Sugar => {
            // бумажный пакет, расширяется книзу
            for y in 3..14 {
                let spread = (y - 3) / 3;
                let (x0, x1) = (5 - spread, 11 + spread);
                for x in x0..=x1 {
                    let c = if x == x0 + 1 && (x + y) % 3 == 0 {
                        idx::TILE_DARK
                    } else {
                        idx::PLASTER
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // сгиб верха
            for x in 5..11 {
                put(&mut s, w, x, 3, idx::TILE);
                put(&mut s, w, x, 4, idx::TILE_DARK);
            }
            // крупинки сахара
            for (x, y) in [(6, 7), (9, 8), (7, 10), (10, 10), (5, 11), (8, 12)] {
                put(&mut s, w, x, y, idx::WHITE);
            }
            // изюм
            for (x, y) in [(8, 7), (6, 9), (9, 11), (11, 9)] {
                put(&mut s, w, x, y, idx::KVASS_DARK);
            }
            // надпись-штрих
            for x in 5..9 {
                put(&mut s, w, x, 13, idx::RED_DARK);
            }
        }
        PropIcon::Bucket => {
            // корпус ведра (трапеция)
            for y in 6..14 {
                let (x0, x1) = (5 - (y - 6) / 4, 10 + (y - 6) / 4);
                for x in x0..=x1 {
                    let c = if x == x1 {
                        idx::METAL_DARK
                    } else {
                        idx::METAL
                    };
                    put(&mut s, w, x, y, c);
                }
            }
            // обод
            for x in 4..12 {
                put(&mut s, w, x, 5, idx::PLASTER);
            }
            // дужка
            for (x, y) in [
                (4, 4),
                (11, 4),
                (4, 3),
                (11, 3),
                (5, 2),
                (10, 2),
                (6, 1),
                (9, 1),
                (7, 1),
                (8, 1),
            ] {
                put(&mut s, w, x, y, idx::METAL_DARK);
            }
            put(&mut s, w, 5, 13, idx::METAL_DARK);
            put(&mut s, w, 6, 14, idx::METAL_DARK);
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
