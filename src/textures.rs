//! РџСЂРѕС†РµРґСѓСЂРЅС‹Рµ С‚РµРєСЃС‚СѓСЂС‹ 64x64 (РёРЅРґРµРєСЃС‹ РїР°Р»РёС‚СЂС‹). Р’СЂРµРјРµРЅРЅРѕ РіРµРЅРµСЂРёСЂСѓСЋС‚СЃСЏ РєРѕРґРѕРј;
//! РїРѕР·Р¶Рµ РјРѕР¶РЅРѕ Р·Р°РјРµРЅРёС‚СЊ РЅР° AI-Р°СЃСЃРµС‚С‹ РёР· РўР— В§2.5.

use crate::palette::idx;
use rand::prelude::*;

pub const TEX_SIZE: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TexId {
    /// РќРёР¶РЅСЏСЏ С‡Р°СЃС‚СЊ СЃС‚РµРЅ вЂ” РєР°С„РµР»СЊ РіР°СЃС‚СЂРѕРЅРѕРјР°.
    TileWall = 0,
    /// Р’РµСЂС…РЅСЏСЏ С‡Р°СЃС‚СЊ СЃС‚РµРЅ вЂ” С€С‚СѓРєР°С‚СѓСЂРєР° + Р±Р°РЅРЅРµСЂ В«РЎР›РђР’Рђ РўР РЈР”РЈ!В».
    PlasterBanner = 1,
    /// Р‘РѕС‡РєР° (СЃС‚РµРЅРєР°).
    Barrel = 2,
    /// РџСЂРёР»Р°РІРѕРє.
    Counter = 3,
    /// РџРѕР» вЂ” Р»РёРЅРѕР»РµСѓРј.
    Floor = 4,
    /// РџРѕС‚РѕР»РѕРє.
    Ceiling = 5,
    /// Р”РІРµСЂРЅРѕР№ РїСЂРѕС‘Рј: РІРёРґ СѓР»РёС†С‹ (РЅРµР±Рѕ/Р»РёСЃС‚РІР°).
    Outside = 6,
    /// РЎС‚РµР»Р»Р°Р¶ СЃ РїРѕСЃСѓРґРѕР№.
    Shelf = 7,
    /// Р’С‹РІРµСЃРєР° В«Р“РђРЎРўР РћРќРћРњВ».
    Sign = 8,
    /// Р¦РµРЅРЅР°СЏ С‚Р°Р±Р»РёС‡РєР°.
    Price = 9,
    /// Р’РёРґ СЃР±РѕРєСѓ Р±РѕС‡РєРё (РґР»СЏ billboard-СЃС‚Р°РЅС†РёР№).
    BarrelSide = 10,
    /// РџРѕР»РєР° СЃ РїР°РєРµС‚Р°РјРё СЃСѓС…Р°СЂРµР№.
    CroutonsShelf = 11,
    /// РџРѕР»РєР° СЃ РїР°С‡РєР°РјРё СЃР°С…Р°СЂР°.
    SugarShelf = 12,
    /// РљРѕР»РѕРЅРєР° СЃ РІРѕРґРѕР№: РєСЂР°РЅ Рё РІРµРґСЂРѕ Сѓ РєР°С„РµР»СЏ.
    WaterTap = 13,
    /// РћРєРЅРѕ СЃРЅР°Р±Р¶РµРЅРёСЏ: С‚С‘РјРЅРѕРµ РѕРєРЅРѕ РІ СЃС‚РµРЅРµ СЃ РґРµСЂРµРІСЏРЅРЅС‹Рј РїРѕРґРѕРєРѕРЅРЅРёРєРѕРј.
    Supply = 14,
    /// РњРѕР№РєР° РїРѕСЃСѓРґС‹: РєР°С„РµР»СЊ, СЂР°РєРѕРІРёРЅР°, РєСЂР°РЅ Рё СЃСѓС€РёР»РєР° СЃРѕ СЃС‚Р°РєР°РЅР°РјРё.
    Sink = 15,
}

pub const NUM_TEX: usize = 16;

pub struct Textures {
    /// data[tex][y*64+x] = РёРЅРґРµРєСЃ РїР°Р»РёС‚СЂС‹
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
        gen_supply(&mut data[TexId::Supply as usize]);
        gen_sink(&mut data[TexId::Sink as usize]);
        Self { data }
    }

    #[inline]
    pub fn texel(&self, t: TexId, x: usize, y: usize) -> u8 {
        self.data[t as usize][(y & 63) * TEX_SIZE + (x & 63)]
    }
}

/// РЁСѓРјРѕРІРѕР№ В«РіСЂСЏР·РЅС‹Р№В» РїРёРєСЃРµР»СЊ: СЃ РІРµСЂРѕСЏС‚РЅРѕСЃС‚СЊСЋ p вЂ” Р·Р°С‚РµРјРЅС‘РЅРЅС‹Р№ РІР°СЂРёР°РЅС‚.
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
                // РїСЏС‚РЅРѕ РіСЂСЏР·Рё
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
            // Р‘Р°РЅРЅРµСЂ В«РЎР›РђР’Рђ РўР РЈР”РЈ!В» вЂ” РєСЂР°СЃРЅР°СЏ РїРѕР»РѕСЃР° РІ СЃСЂРµРґРЅРµР№ С‚СЂРµС‚Рё.
            let banner = y >= 20 && y < 44;
            if banner {
                let edge = y == 20 || y == 43;
                let c = if edge {
                    idx::BLACK
                } else if y == 22 || y == 41 {
                    // СѓСЃР»РѕРІРЅС‹Р№ В«С‚РµРєСЃС‚В»: Р±РµР»С‹Рµ С€С‚СЂРёС…Рё
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
            // С†РёР»РёРЅРґСЂ: РєСЂР°СЏ С‚РµРјРЅРµРµ, СЃРµСЂРµРґРёРЅР° СЃРІРµС‚Р»РµРµ (Р±РѕС‡РєСѓ РІРёРґРЅРѕ В«РІ РєСЂСѓРіРµВ»)
            let nx = (x as f32 - TEX_SIZE as f32 / 2.0) / (TEX_SIZE as f32 / 2.0);
            let rim = 1.0 - nx * nx; // 0 РїРѕ РєСЂР°СЏРј, 1 РІ С†РµРЅС‚СЂРµ
            // РѕР±СЂСѓС‡ СЃ Р»С‘РіРєРёРј РёР·РіРёР±РѕРј (РїРѕ С†РёР»РёРЅРґСЂСѓ РІС‹С€Рµ/РЅРёР¶Рµ С†РµРЅС‚СЂР°)
            let bow = (nx * nx * 6.0) as usize;
            let hoop = (8 + bow..13 + bow).contains(&y) || (51 + bow..56 + bow).contains(&y);
            let c = if hoop {
                // РјРµС‚Р°Р»Р» СЃ Р±Р»РёРєРѕРј РїРѕСЃРµСЂРµРґРёРЅРµ
                if rim > 0.8 {
                    idx::WHITE
                } else if rim > 0.45 {
                    idx::METAL
                } else {
                    idx::METAL_DARK
                }
            } else {
                // РґРѕСЃРєРё-СЃС‚РѕР№РєРё: С‚РѕРЅРєРёРµ С€РІС‹, РґРѕСЃРєР° С‡СѓС‚СЊ СЃРІРµС‚Р»РµРµ РІ С†РµРЅС‚СЂРµ
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

    // РєСЂР°РЅ РІРЅРёР·Сѓ РїРѕ С†РµРЅС‚СЂСѓ: РІРµРЅС‚РёР»СЊ, РєРѕСЂРїСѓСЃ, РЅРѕСЃРёРє СЃ Р±Р»РёРєРѕРј
    for y in 42..47 {
        for x in 28..36 {
            px(t, x, y, if y == 42 || x == 28 || x == 35 { idx::BLACK } else { idx::METAL });
        }
    }
    for y in 47..54 {
        for x in 30..34 {
            px(t, x, y, if x == 33 { idx::WHITE } else { idx::METAL_DARK });
        }
    }
    for y in 50..52 {
        for x in 34..39 {
            px(t, x, y, if y == 51 { idx::WHITE } else { idx::METAL });
        }
    }
    // РІРµРЅС‚РёР»СЊ СЃРІРµСЂС…Сѓ
    for x in 25..39 {
        px(t, x, 40, idx::METAL_DARK);
        px(t, x, 41, idx::METAL);
    }
}

/// РџСЂРёР»Р°РІРѕРє. Р Р°РЅСЊС€Рµ РїРѕ РІСЃРµР№ С€РёСЂРёРЅРµ С€Р»Рё С‡С‘СЂРЅС‹Рµ РІРµСЂС‚РёРєР°Р»СЊРЅС‹Рµ С€РІС‹ РєР°Р¶РґС‹Рµ 16px,
/// РёР·-Р·Р° РєРѕС‚РѕСЂС‹С… РѕРґРёРЅ СЃРїР»РѕС€РЅРѕР№ РїСЂРёР»Р°РІРѕРє С‡РёС‚Р°Р»СЃСЏ РєР°Рє РЅРµСЃРєРѕР»СЊРєРѕ РѕС‚РґРµР»СЊРЅС‹С… Р±СѓРґРѕРє.
/// РўРµРїРµСЂСЊ вЂ” СЃРїР»РѕС€РЅР°СЏ РіРѕСЂРёР·РѕРЅС‚Р°Р»СЊРЅР°СЏ РєРѕРЅСЃС‚СЂСѓРєС†РёСЏ: РєСЂРѕРјРєР°, РµРґРёРЅР°СЏ РєСЂР°СЃРЅР°СЏ
/// Р»РµРЅС‚Р° В«РљР’РђРЎВ» Рё С†РѕРєРѕР»СЊ; РІРµСЂС‚РёРєР°Р»СЊ С‚РѕР»СЊРєРѕ РІ РІРёРґРµ РјСЏРіРєРѕРіРѕ РґСЂРµРІРµСЃРЅРѕРіРѕ С€РІР°.
fn gen_counter(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(4);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let c = if y < 7 {
                // РєСЂРѕРјРєР° РїСЂРёР»Р°РІРєР° вЂ” СЃРІРµС‚Р»РѕРµ РґРµСЂРµРІРѕ, РІРѕР»РѕРєРЅР° РїРѕ РіРѕСЂРёР·РѕРЅС‚Р°Р»Рё
                if y == 0 {
                    idx::PLASTER
                } else if y < 3 {
                    noisy(&mut rng, idx::WOOD_LIGHT, idx::PLASTER, 0.1)
                } else {
                    noisy(&mut rng, idx::WOOD_LIGHT, idx::WOOD_DARK, 0.1)
                }
            } else if y == 7 || y == 8 {
                // С‚РµРЅСЊ РїРѕРґ РєСЂРѕРјРєРѕР№ вЂ” РґРµСЂР¶РёС‚ РµРґРёРЅСЃС‚РІРѕ РЅР° РІСЃСЋ РґР»РёРЅСѓ
                if y == 7 { idx::WOOD_DARK } else { idx::SHADOW }
            } else if y >= 52 {
                // С†РѕРєРѕР»СЊ вЂ” С‚С‘РјРЅРѕРµ РґРµСЂРµРІРѕ РґРѕ РїРѕР»Р°
                if y == 52 {
                    idx::WOOD_DARK
                } else {
                    noisy(&mut rng, idx::WOOD_DARK, idx::SHADOW, 0.1)
                }
            } else {
                // С‡РёСЃС‚С‹Р№ РґРµСЂРµРІСЏРЅРЅС‹Р№ С„Р°СЃР°Рґ (РўР—: Р±РµР· РєСЂР°СЃРЅС‹С… Р»РµРЅС‚ Рё Р·Р°РїР»Р°С‚РѕРє)
                let seam = x % 21 == 20;
                if seam || y % 7 == 0 {
                    idx::WOOD_DARK
                } else {
                    noisy(&mut rng, idx::WOOD_LIGHT, idx::WOOD_DARK, 0.1)
                }
            };
            px(t, x, y, c);
        }
    }

    // РљР°СЃСЃРѕРІС‹Р№ Р°РїРїР°СЂР°С‚ РЅР° С„Р°СЃР°РґРµ (РўР—): РїРѕ С†РµРЅС‚СЂСѓ РїСЂРёР»Р°РІРєР° вЂ” РєРѕСЂРїСѓСЃ, РґРёСЃРїР»РµР№
    // СЃ СЃСѓРјРјРѕР№, РїРѕР»Рµ РєР»Р°РІРёС€ Рё С‡РµРєРѕРІР°СЏ Р»РµРЅС‚Р° СЃРїСЂР°РІР°.
    for y in 9..26 {
        for x in 16..48 {
            let c = if y == 9 || x == 16 || x == 47 || y == 25 {
                idx::BLACK
            } else if x >= 22 && x <= 40 && (10..14).contains(&y) {
                // РґРёСЃРїР»РµР№: С‚С‘РјРЅС‹Р№ С„РѕРЅ СЃ СЏРЅС‚Р°СЂРЅРѕР№ СЃСѓРјРјРѕР№
                if y == 11 && x >= 25 && x <= 36 {
                    idx::LAMP
                } else {
                    idx::SHADOW
                }
            } else if (15..24).contains(&y)
                && x >= 18
                && (x - 18) % 6 >= 1
                && (x - 18) % 6 <= 4
            {
                // РїРѕР»Рµ РєР»Р°РІРёС€
                let row = y / 5;
                let col_key = (x - 18) / 6;
                let key_on = (row + col_key) % 2 == 0;
                if y % 5 == 4 {
                    idx::BLACK
                } else if key_on {
                    idx::LAMP
                } else {
                    idx::PLASTER
                }
            } else if x >= 41 {
                // СЃРїСЂР°РІР° вЂ” С‡РµРєРѕРІР°СЏ Р»РµРЅС‚Р°
                if (12..23).contains(&y) {
                    idx::WHITE
                } else {
                    idx::METAL
                }
            } else {
                idx::METAL
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
            // РѕРґРЅР° Р»Р°РјРїРѕРІР°СЏ РїР°РЅРµР»СЊ РЅР° С‚Р°Р№Р» вЂ” СЃРІРµС‚Р»С‹Рµ РїРѕР»РѕСЃС‹, РЅРµС‡Р°СЃС‚Рѕ
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
                // РЅРµР±Рѕ
                noisy(&mut rng, idx::SKY, idx::WHITE, 0.08)
            } else if y < 44 {
                // РєСЂРѕРЅС‹ Р»РёРїС‹
                noisy(&mut rng, idx::LEAVES, idx::GREEN, 0.5)
            } else {
                // Р»Р°РІРѕС‡РєР°/С‚СЂРѕС‚СѓР°СЂ
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
                // Р±Р°РЅРєРё/СЃС‚Р°РєР°РЅС‹ РЅР° РїРѕР»РєРµ
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
                // В«Р“РђРЎРўР РћРќРћРњВ» вЂ” СѓСЃР»РѕРІРЅС‹Рµ Р±СѓРєРІС‹: Р±РµР»С‹Рµ Р±Р»РѕРєРё РїРѕ С†РµРЅС‚СЂСѓ
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
                // СЃС‚СЂРѕРєРё В«С†РµРЅВ»
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

/// РћР±С‰Р°СЏ РїРѕР»РєР°: С‚СЂРё СЏСЂСѓСЃР° РґРѕСЃРѕРє СЃ С‚РѕРІР°СЂРѕРј РјРµР¶РґСѓ РЅРёРјРё.
fn shelf_bg(x: usize, y: usize) -> Option<u8> {
    let cell = y % 21;
    if cell < 3 {
        return Some(if cell == 0 { idx::WOOD_DARK } else { idx::WOOD_LIGHT });
    }
    None
}

/// РџРѕР»РєР° СЃСѓС…Р°СЂРµР№: РєРѕСЂРёС‡РЅРµРІС‹Рµ РїР°РєРµС‚С‹ СЃ РѕРєРѕС€РєРѕРј.
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
                        // РѕРєРѕС€РєРѕ СЃ СЃСѓС…Р°СЂСЏРјРё
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

/// РџРѕР»РєР° СЃР°С…Р°СЂР°: Р±РµР»С‹Рµ РїР°С‡РєРё СЃ РєСЂР°СЃРЅРѕР№ РїРѕР»РѕСЃРѕР№.
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

/// РљРѕР»РѕРЅРєР° СЃ РІРѕРґРѕР№: РєР°С„РµР»СЊ, РєСЂР°РЅ Рё РІРµРґСЂРѕ.
fn gen_water(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(11);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // РєР°С„РµР»СЊРЅР°СЏ СЃС‚РµРЅР°
            let grout = x % 16 == 0 || y % 16 == 0;
            let mut c = if grout {
                idx::TILE_DARK
            } else {
                noisy(&mut rng, idx::TILE, idx::PLASTER, 0.08)
            };
            // С‚СЂСѓР±Р°
            if (28..36).contains(&x) && y < 44 {
                c = if x == 28 || x == 35 {
                    idx::BLACK
                } else if x == 30 {
                    idx::WHITE
                } else {
                    idx::METAL
                };
            }
            // РєСЂР°РЅ СЃ РЅРѕСЃРёРєРѕРј
            if (36..46).contains(&x) && (32..37).contains(&y) {
                c = idx::METAL_DARK;
            }
            if (42..46).contains(&x) && (37..43).contains(&y) {
                c = idx::METAL;
            }
            // РІРµРґСЂРѕ РІРЅРёР·Сѓ
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

/// РћРєРЅРѕ СЃРЅР°Р±Р¶РµРЅРёСЏ (РўР— в„–3): С‚С‘РјРЅРѕРµ РѕРєРЅРѕ РІ С€С‚СѓРєР°С‚СѓСЂРЅРѕР№ СЃС‚РµРЅРµ СЃ РґРµСЂРµРІСЏРЅРЅС‹Рј
/// РїРѕРґРѕРєРѕРЅРЅРёРєРѕРј Рё РєСЂР°СЃРЅРѕР№ С‚Р°Р±Р»РёС‡РєРѕР№ В«РЎРќРђР‘В» (СѓСЃР»РѕРІРЅРѕР№ вЂ” Р±РµР· Р±СѓРєРІ).
fn gen_supply(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(21);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // С„РѕРЅ вЂ” С€С‚СѓРєР°С‚СѓСЂРєР°
            let mut c = noisy(&mut rng, idx::PLASTER, idx::TILE_DARK, 0.10);
            // РѕРєРѕРЅРЅС‹Р№ РїСЂРѕС‘Рј
            if (12..52).contains(&x) && (8..40).contains(&y) {
                // СЂР°РјР°
                let frame = x == 12 || x == 51 || y == 8 || y == 39;
                c = if frame {
                    idx::WOOD_DARK
                } else if (16..48).contains(&x) && (12..36).contains(&y) {
                    // С‚С‘РјРЅРѕРµ СЃС‚РµРєР»Рѕ СЃ Р»С‘РіРєРёРј Р±Р»РёРєРѕРј
                    if x + y == 40 || x + y == 41 {
                        idx::METAL
                    } else {
                        idx::SHADOW
                    }
                } else {
                    idx::WOOD_LIGHT
                };
            }
            // РїРѕРґРѕРєРѕРЅРЅРёРє
            if (8..56).contains(&x) && (40..45).contains(&y) {
                c = if y == 40 { idx::WOOD_LIGHT } else { idx::WOOD_DARK };
            }
            // РєСЂР°СЃРЅР°СЏ С‚Р°Р±Р»РёС‡РєР° В«РЎРќРђР‘В» РЅР°Рґ РѕРєРЅРѕРј
            if (18..46).contains(&x) && (1..6).contains(&y) {
                c = if y == 1 || y == 5 || x == 18 || x == 45 {
                    idx::BLACK
                } else if (x + 2) % 6 < 2 {
                    idx::WHITE
                } else {
                    idx::RED
                };
            }
            // РјРµС€РєРё СЃ РјСѓРєРѕР№ Сѓ СЃС‚РµРЅС‹ РІРЅРёР·Сѓ
            if (6..20).contains(&x) && (48..60).contains(&y) {
                c = if (6..20).contains(&x) && y == 48 {
                    idx::WOOD_DARK
                } else {
                    noisy(&mut rng, idx::WOOD_LIGHT, idx::PLASTER, 0.3)
                };
            }
            px(t, x, y, c);
        }
    }
}

/// РњРѕР№РєР° РїРѕСЃСѓРґС‹ (РўР— в„–8): РєР°С„РµР»СЊ, РјРµС‚Р°Р»Р»РёС‡РµСЃРєР°СЏ СЂР°РєРѕРІРёРЅР°, РєСЂР°РЅ,
/// СЃСѓС€РёР»РєР° СЃ РїРµСЂРµРІС‘СЂРЅСѓС‚С‹РјРё СЃС‚Р°РєР°РЅР°РјРё.
fn gen_sink(t: &mut Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(31);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            // РєР°С„РµР»СЊРЅР°СЏ СЃС‚РµРЅР°
            let grout = x % 16 == 0 || y % 16 == 0;
            let mut c = if grout {
                idx::TILE_DARK
            } else {
                noisy(&mut rng, idx::TILE, idx::PLASTER, 0.08)
            };
            // РєСЂР°РЅ СЃРІРµСЂС…Сѓ
            if (28..36).contains(&x) && y < 30 {
                c = if x == 28 || x == 35 {
                    idx::BLACK
                } else if x == 30 {
                    idx::WHITE
                } else {
                    idx::METAL
                };
            }
            if (30..38).contains(&x) && (28..33).contains(&y) {
                c = idx::METAL_DARK;
            }
            // СЃС‚СЂСѓСЏ РІРѕРґС‹
            if (33..36).contains(&x) && (33..42).contains(&y) {
                c = if x == 34 { idx::WHITE } else { idx::SKY };
            }
            // СЂР°РєРѕРІРёРЅР°
            if (10..54).contains(&x) && (42..62).contains(&y) {
                c = if y == 42 || x == 10 || x == 53 || y == 61 {
                    idx::BLACK
                } else if y == 44 {
                    idx::METAL_DARK
                } else if x == 13 {
                    idx::WHITE
                } else if y > 55 {
                    // РІРѕРґР° РІ СЂР°РєРѕРІРёРЅРµ
                    if (x + y) % 3 == 0 {
                        idx::SKY
                    } else {
                        idx::METAL_DARK
                    }
                } else {
                    idx::METAL
                };
            }
            // РїРµСЂРµРІС‘СЂРЅСѓС‚С‹Рµ СЃС‚Р°РєР°РЅС‹ РЅР° СЃСѓС€РёР»РєРµ (СЃР»РµРІР°)
            for (gx, gy) in [(16, 34), (24, 34), (32, 34)] {
                if (gx..gx + 5).contains(&x) && (gy..gy + 8).contains(&y) {
                    c = if x == gx || x == gx + 4 || y == gy + 7 {
                        idx::TILE_DARK
                    } else if x == gx + 1 {
                        idx::WHITE
                    } else {
                        idx::TILE
                    };
                }
            }
            px(t, x, y, c);
        }
    }
}
