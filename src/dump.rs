//! Дамп процедурных спрайтов в PNG (без внешних кодеков).
//! Запуск: `kvass-shop --dump-sprites [каталог]` — пишет npc_<имя>.png
//! и лист npc_sheet.png, увеличенные ×8, чтобы пиксели были видны.

use crate::palette::{idx, PALETTE};
use crate::sprites::{SpriteSheet, NpcKind, SPR_H, SPR_W, TRANSPARENT};

// ---------- минимальный PNG-энкодер (RGB8, без сжатия) ----------

fn crc32(buf: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in buf {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(kind);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// Строка фильтра 0 + zlib-поток из Stored-блоков deflate.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut i = 0;
    while i < raw.len() {
        let n = (raw.len() - i).min(0xFFFF);
        let last = i + n >= raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&raw[i..i + n]);
        i += n;
    }
    // adler32 по исходным данным
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in raw {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// rgb — 3 байта на пиксель, размеры уже увеличены.
pub fn write_png(path: &std::path::Path, w: usize, h: usize, rgb: &[u8]) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for y in 0..h {
        raw.push(0); // фильтр None
        raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]);
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 бит, truecolor
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    std::fs::write(path, out)
}

// ---------- сами дампы ----------

fn upscale(spr: &[u8], scale: usize) -> (usize, usize, Vec<u8>) {
    let w = SPR_W * scale;
    let h = SPR_H * scale;
    let mut rgb = vec![0u8; w * h * 3];
    for y in 0..SPR_H {
        for x in 0..SPR_W {
            let c = spr[y * SPR_W + x];
            let px = if c == TRANSPARENT {
                [255, 0, 255] // прозрачность — пурпуром, чтобы её было видно
            } else {
                PALETTE[c as usize]
            };
            for dy in 0..scale {
                for dx in 0..scale {
                    let i = ((y * scale + dy) * w + (x * scale + dx)) * 3;
                    rgb[i..i + 3].copy_from_slice(&px);
                }
            }
        }
    }
    (w, h, rgb)
}

pub fn run() {
    let dir = std::env::args()
        .nth(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let sheet = SpriteSheet::generate();
    let kinds = [
        NpcKind::Grandma,
        NpcKind::Worker,
        NpcKind::Mama,
        NpcKind::Student,
        NpcKind::Militsioner,
        NpcKind::Hooligan,
        NpcKind::Sanepid,
    ];
    for k in &kinds {
        let (w, h, rgb) = upscale(sheet.npc_sprite(*k), 8);
        let p = dir.join(format!("npc_{}.png", k.name()));
        write_png(&p, w, h, &rgb).expect("не удалось записать PNG");
        println!("записан {}", p.display());
    }
    // лист: 7 спрайтов в ряд, фон — пурпур (прозрачность)
    let scale = 8;
    let gap = 8;
    let sw = (SPR_W * scale + gap) * kinds.len() + gap;
    let sh = SPR_H * scale + gap * 2;
    let mut rgb = vec![255u8; sw * sh * 3];
    for (i, k) in kinds.iter().enumerate() {
        let spr = sheet.npc_sprite(*k);
        let ox = gap + i * (SPR_W * scale + gap);
        for y in 0..SPR_H * scale {
            for x in 0..SPR_W * scale {
                let c = spr[(y / scale) * SPR_W + (x / scale)];
                if c == TRANSPARENT {
                    continue;
                }
                let px = PALETTE[c as usize];
                let idx = ((y + gap) * sw + ox + x) * 3;
                rgb[idx..idx + 3].copy_from_slice(&px);
            }
        }
    }
    let p = dir.join("npc_sheet.png");
    write_png(&p, sw, sh, &rgb).expect("не удалось записать лист");
    println!("записан {}", p.display());
    // для отладки: константы и первые пиксели лица бабушки
    let g = sheet.npc_sprite(NpcKind::Grandma);
    println!(
        "размер спрайта {SPR_W}x{SPR_H}, индексов палитры: {}, transparent={TRANSPARENT}",
        PALETTE.len()
    );
    let _ = idx::SKIN;
}
