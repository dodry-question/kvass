//! Процедурные текстуры 64x64 (индексы палитры). Временно генерируются кодом;
//! позже можно заменить на AI-ассеты из ТЗ §2.5.

use crate::palette::idx;
use rand::prelude::*;

pub const TEX_SIZE: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TexId {
    /// Нижняя часть стен — кафель гастронома.
    TileWall = 0,
    /// Верхняя часть стен — штукатурка + баннер «СЛАВА ТРУДУ!».
    PlasterBanner = 1,
    /// Бочка (стенка).
    Barrel = 2,
    /// Прилавок.
    Counter = 3,
    /// Пол — линолеум.
    Floor = 4,
    /// Потолок.
    Ceiling = 5,
    /// Дверной проём: вид улицы (небо/листва).
    Outside = 6,
    /// Стеллаж с посудой.
    Shelf = 7,
    /// Вывеска «ГАСТРОНОМ».
    Sign = 8,
    /// Ценная табличка.
    Price = 9,
    /// Вид сбоку бочки (для billboard-станций).
    BarrelSide = 10,
    /// Полка с пакетами сухарей.
    CroutonsShelf = 11,
    /// Полка с пачками сахара.
    SugarShelf = 12,
    /// Колонка с водой: кран и ведро у кафеля.
    WaterTap = 13,
}

pub const NUM_TEX: usize = 14;

pub struct Textures {
    /// data[tex][y*64+x] = индекс палитры
    pub data: Vec<Vec<u8>>,
}

fn px(tex: &mut Vec<u8>, x: usize, y: usize, c: u8) {
    tex[y * TEX_SIZE + x] = c;
}

impl Textures {
    pub fn generate() -> Self {
        let mut data = vec![vec![0u8; TEX_SIZE * TEX_SIZE]; NUM_TEX];
        gen_tile_wall(&mut data[TexId::TileWall as usize]);
        gen_plaster(&mut data[TexId::PlasterBanner as usize]);
        gen_barrel(&mut data[TexId::Barrel as usize]);
        gen_counter(&mut data[TexId::Counter as usize]);
        gen_floor(&mut data[TexId::Floor as usize]);
        gen_ceiling(&mut data[TexId::Ceiling as usize]);
        gen_outside(&mut data[TexId::Outside as usize]);
        gen_shelf(&mut data[TexId::Shelf as usize]);
        gen_sign(&mut data[TexId::Sign as usize]);
        gen_price(&mut data[TexId::Price as usize]);
        gen_barrel(&mut data[TexId::BarrelSide as usize]);
        gen_croutons(&mut data[TexId::CroutonsShelf as usize]);
        gen_sugar(&mut data[TexId::SugarShelf as usize]);
        gen_water(&mut data[TexId::WaterTap as usize]);
        Self { data }
    }

    #[inline]
    pub fn texel(&self, t: TexId, x: usize, y: usize) -> u8 {
        self.data[t as usize][(y & 63) * TEX_SIZE + (x & 63)]
    }
}

/// Шумовой «грязный» пиксель: с вероятностью p — затемнённый вариант.
fn noisy(rng: &mut StdRng, base: u8, dark: u8, p: f32) -> u8 {
    if rng.random::<f32>() < p {
        dark
    } else {
        base
    }
}

fn gen_tile_wall(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(1);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let grout = x % 16 == 0 || y % 16 == 0;
            let c = if grout {
                idx::TILE_DARK
            } else {
                // пятно грязи
                let dirt = ((x / 8) * 7 + (y / 8) * 3) % 11 == 0;
                if dirt {
                    idx::TILE_DARK
                } else {
                    noisy(&mut rng, idx::TILE, idx::TILE_DARK, 0.12)
                }
            };
            px(t, x, y, c);
        }
    }
}

fn gen_plaster(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(2);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // Баннер «СЛАВА ТРУДУ!» — красная полоса в средней трети.
            let banner = y >= 20 && y < 44;
            if banner {
                let edge = y == 20 || y == 43;
                let c = if edge {
                    idx::BLACK
                } else if y == 22 || y == 41 {
                    // условный «текст»: белые штрихи
                    if (x / 3) % 2 == 0 {
                        idx::WHITE
                    } else {
                        idx::RED_DARK
                    }
                } else {
                    noisy(&mut rng, idx::RED, idx::RED_DARK, 0.1)
                };
                px(t, x, y, c);
            } else {
                let c = noisy(&mut rng, idx::PLASTER, idx::TILE_DARK, 0.08);
                px(t, x, y, c);
            }
        }
    }
}

fn gen_barrel(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(3);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // цилиндр: края темнее, середина светлее (бочку видно «в круге»)
            let nx = (x as f32 - TEX_SIZE as f32 / 2.0) / (TEX_SIZE as f32 / 2.0);
            let rim = 1.0 - nx * nx; // 0 по краям, 1 в центре
            // обруч с лёгким изгибом (по цилиндру выше/ниже центра)
            let bow = (nx * nx * 6.0) as usize;
            let hoop = (8 + bow..13 + bow).contains(&y) || (51 + bow..56 + bow).contains(&y);
            let c = if hoop {
                // металл с бликом посередине
                if rim > 0.8 {
                    idx::WHITE
                } else if rim > 0.45 {
                    idx::METAL
                } else {
                    idx::METAL_DARK
                }
            } else {
                // доски-стойки: тонкие швы, доска чуть светлее в центре
                let stave = x % 7 == 6;
                if stave {
                    idx::BLACK
                } else if rim > 0.75 {
                    noisy(&mut rng, idx::WOOD_LIGHT, idx::WOOD_DARK, 0.12)
                } else if rim > 0.35 {
                    noisy(&mut rng, idx::WOOD_DARK, idx::WOOD_LIGHT, 0.15)
                } else {
                    noisy(&mut rng, idx::SHADOW, idx::WOOD_DARK, 0.1)
                }
            };
            px(t, x, y, c);
        }
    }
}

fn gen_counter(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(4);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if y < 8 {
                // кромка прилавка — светлое дерево
                noisy(&mut rng, idx::WOOD_LIGHT, idx::WOOD_DARK, 0.08)
            } else if y == 8 || y == 55 {
                idx::BLACK
            } else if y >= 56 {
                // цоколь
                noisy(&mut rng, idx::WOOD_DARK, idx::SHADOW, 0.1)
            } else if (26..34).contains(&y) {
                // красная роспись «КВАС» с белыми штрихами
                if y == 27 || y == 32 {
                    if (x / 4) % 2 == 0 {
                        idx::WHITE
                    } else {
                        idx::RED
                    }
                } else {
                    noisy(&mut rng, idx::RED, idx::RED_DARK, 0.1)
                }
            } else if x % 16 == 0 {
                // вертикальные доски фасада
                idx::BLACK
            } else {
                noisy(&mut rng, idx::WOOD_LIGHT, idx::WOOD_DARK, 0.1)
            };
            px(t, x, y, c);
        }
    }
}

fn gen_floor(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(5);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if x % 32 == 0 || y % 32 == 0 {
                idx::SHADOW
            } else {
                noisy(&mut rng, idx::FLOOR, idx::SHADOW, 0.1)
            };
            px(t, x, y, c);
        }
    }
}

fn gen_ceiling(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(6);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // одна ламповая панель на тайл — светлые полосы, нечасто
            let lamp = (12..52).contains(&x) && (14..26).contains(&y);
            let c = if lamp {
                let frame = x == 12 || y == 14 || x == 51 || y == 25;
                if frame {
                    idx::METAL_DARK
                } else {
                    noisy(&mut rng, idx::LAMP, idx::PLASTER, 0.12)
                }
            } else {
                noisy(&mut rng, idx::CEILING, idx::SHADOW, 0.15)
            };
            px(t, x, y, c);
        }
    }
}

fn gen_outside(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(7);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if y < 34 {
                // небо
                noisy(&mut rng, idx::SKY, idx::WHITE, 0.08)
            } else if y < 44 {
                // кроны липы
                noisy(&mut rng, idx::LEAVES, idx::GREEN, 0.5)
            } else {
                // лавочка/тротуар
                let bench = (20..44).contains(&x) && (46..54).contains(&y);
                if bench {
                    idx::WOOD_DARK
                } else {
                    noisy(&mut rng, idx::TILE_DARK, idx::SHADOW, 0.2)
                }
            };
            px(t, x, y, c);
        }
    }
}

fn gen_shelf(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(8);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let plank = y % 20 < 3;
            let c = if plank {
                if y % 20 == 0 {
                    idx::WOOD_DARK
                } else {
                    idx::WOOD_LIGHT
                }
            } else {
                // банки/стаканы на полке
                let jar = (x % 16) < 8 && ((y % 20) % 11) > 4;
                if jar {
                    if (x % 16) % 2 == 0 {
                        idx::METAL
                    } else {
                        idx::BOTTLE_GLASS
                    }
                } else {
                    noisy(&mut rng, idx::SHADOW, idx::BLACK, 0.2)
                }
            };
            px(t, x, y, c);
        }
    }
}

fn gen_sign(t: &mut Vec<u8>) {
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let border = x < 4 || x >= 60 || y < 4 || y >= 60;
            let c = if border {
                idx::WHITE
            } else {
                // «ГАСТРОНОМ» — условные буквы: белые блоки по центру
                let in_text_y = (24..40).contains(&y);
                let in_text_x = (6..58).contains(&x);
                let glyph = in_text_y
                    && in_text_x
                    && ((x / 5) % 2 == 0 || y == 24 || y == 31 || y == 39);
                if glyph {
                    idx::WHITE
                } else {
                    idx::RED
                }
            };
            px(t, x, y, c);
        }
    }
}

fn gen_price(t: &mut Vec<u8>) {
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if x < 2 || x >= 62 || y < 2 || y >= 62 {
                idx::WOOD_DARK
            } else {
                // строки «цен»
                let line = [12, 28, 44].iter().any(|&ly| (ly..ly + 8).contains(&y));
                if line && x >= 8 && x < 56 && (x / 4) % 2 == 0 {
                    idx::BLACK
                } else {
                    idx::PLASTER
                }
            };
            px(t, x, y, c);
        }
    }
}

/// Общая полка: три яруса досок с товаром между ними.
fn shelf_bg(x: usize, y: usize) -> Option<u8> {
    let cell = y % 21;
    if cell < 3 {
        return Some(if cell == 0 { idx::WOOD_DARK } else { idx::WOOD_LIGHT });
    }
    None
}

/// Полка сухарей: коричневые пакеты с окошком.
fn gen_croutons(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(9);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if let Some(c) = shelf_bg(x, y) {
                c
            } else {
                let bag = (x % 21) >= 4 && (x % 21) <= 16 && (y % 21) >= 6;
                if bag {
                    let cx = x % 21;
                    let cy = y % 21;
                    if cx == 4 || cx == 16 || cy == 6 || cy == 20 {
                        idx::BLACK
                    } else if (9..=12).contains(&cx) && (9..=13).contains(&cy) {
                        // окошко с сухарями
                        if (cx + cy) % 2 == 0 {
                            idx::WOOD_LIGHT
                        } else {
                            idx::WOOD_DARK
                        }
                    } else {
                        noisy(&mut rng, idx::WOOD_DARK, idx::SHADOW, 0.08)
                    }
                } else {
                    noisy(&mut rng, idx::SHADOW, idx::BLACK, 0.15)
                }
            };
            px(t, x, y, c);
        }
    }
}

/// Полка сахара: белые пачки с красной полосой.
fn gen_sugar(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(10);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if let Some(c) = shelf_bg(x, y) {
                c
            } else {
                let pack = (x % 21) >= 4 && (x % 21) <= 16 && (y % 21) >= 5;
                if pack {
                    let cx = x % 21;
                    let cy = y % 21;
                    if cx == 4 || cx == 16 || cy == 5 || cy == 20 {
                        idx::BLACK
                    } else if (10..=12).contains(&cy) {
                        idx::RED
                    } else if (6..=8).contains(&cy) && (cx + 1) % 3 == 0 {
                        idx::RED_DARK
                    } else {
                        noisy(&mut rng, idx::PLASTER, idx::TILE, 0.06)
                    }
                } else {
                    noisy(&mut rng, idx::SHADOW, idx::BLACK, 0.15)
                }
            };
            px(t, x, y, c);
        }
    }
}

/// Колонка с водой: кафель, кран и ведро.
fn gen_water(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(11);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // кафельная стена
            let grout = x % 16 == 0 || y % 16 == 0;
            let mut c = if grout {
                idx::TILE_DARK
            } else {
                noisy(&mut rng, idx::TILE, idx::PLASTER, 0.08)
            };
            // труба
            if (28..36).contains(&x) && y < 44 {
                c = if x == 28 || x == 35 {
                    idx::BLACK
                } else if x == 30 {
                    idx::WHITE
                } else {
                    idx::METAL
                };
            }
            // кран с носиком
            if (36..46).contains(&x) && (32..37).contains(&y) {
                c = idx::METAL_DARK;
            }
            if (42..46).contains(&x) && (37..43).contains(&y) {
                c = idx::METAL;
            }
            // ведро внизу
            if (24..44).contains(&x) && (46..60).contains(&y) {
                c = if y == 46 || x == 24 || x == 43 {
                    idx::BLACK
                } else if y == 48 {
                    idx::METAL_DARK
                } else if x == 27 {
                    idx::WHITE
                } else {
                    idx::METAL
                };
            }
            px(t, x, y, c);
        }
    }
}
