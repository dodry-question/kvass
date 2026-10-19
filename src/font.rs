//! Текст на RGBA-канвасе UI.
//!
//! Шрифт — обычный кириллический TTF (assets/fonts/golos.ttf), растеризуется
//! fontdue в полутоновые маски (антиалиасинг). Кегли — фиксированный набор
//! SIZES, поэтому ширину/высоту строк можно измерять, а вёрстку панелей
//! вести по метрикам — строки не наезжают друг на друга.
//! Метрики fontdue: ymin — низ растра над базовой линией (может быть < 0),
//! верхняя строка растра — на ymin+h-1 выше базовой линии.
//! Если TTF нет — запасной встроенный растровый шрифт 5×6 (только заглавные).

use std::collections::HashMap;

use crate::canvas::Canvas;

/// Набор кеглей (пиксели канваса 1280×800).
pub const SIZES: [usize; 6] = [20, 24, 30, 38, 48, 64];
pub const SZ_S: usize = 20;
pub const SZ_M: usize = 24;
pub const SZ_L: usize = 30;
pub const SZ_XL: usize = 38;
pub const SZ_T: usize = 48;
pub const SZ_H: usize = 64;

/// Ближайший кегль не больше запрошенного.
pub fn pick(px: usize) -> usize {
    let mut best = SIZES[0];
    for &s in SIZES.iter() {
        if s <= px {
            best = s;
        }
    }
    best
}

/// Самый близкий кегль.
fn pick_nearest(px: usize) -> usize {
    let mut best = SIZES[0];
    let mut bd = usize::MAX;
    for &s in SIZES.iter() {
        let d = s.abs_diff(px);
        if d < bd {
            bd = d;
            best = s;
        }
    }
    best
}

fn norm_char(c: char) -> char {
    match c {
        '—' | '–' | '−' => '-',
        '…' => '.',
        _ => c,
    }
}

/// Есть ли глиф в растровой карте (непустой).
fn raster_missing(r: &HashMap<char, Glyph>, c: char) -> bool {
    r.get(&c).map_or(true, |g| g.w == 0)
}

/// Полутоновая маска глифа TTF (антиалиасинг).
#[derive(Clone)]
struct Glyph {
    /// Шаг каретки.
    adv: i32,
    /// Смещение чернил от каретки по X.
    xmin: i32,
    /// Низ растра над базовой линией (может быть отрицательным).
    ymin: i32,
    w: usize,
    h: usize,
    /// Покрытие 0..255, строки сверху вниз.
    cov: Vec<u8>,
}

/// Метрики кегля: (капс, выносные, высота строки, воздух сверху).
type SizeMetrics = (i32, i32, i32, i32);

pub struct Font {
    fallback: HashMap<char, [u8; GLYPH_H]>,
    /// Кегль -> глифы.
    sizes: HashMap<usize, HashMap<char, Glyph>>,
    /// Кегль -> метрики.
    metrics: HashMap<usize, SizeMetrics>,
    /// Есть ли настоящий TTF (иначе запасной 5×6).
    ttf: bool,
    /// Отладочный кегль (печатается в selftest).
    pub px_label: String,
}

pub const GLYPH_W: usize = 5;
pub const GLYPH_H: usize = 6;

pub const GLYPHS: &[(char, [&str; 6])] = &[
    // --- Кириллица (запасной) ---
    ('А', [".###.", "#...#", "#####", "#...#", "#...#", "...."]),
    ('Б', ["#####", "#....", "#####", "....#", "#####", "...."]),
    ('В', ["####.", "#...#", "####.", "#...#", "####.", "...."]),
    ('Г', ["#####", "#....", "#....", "#....", "#....", "...."]),
    ('Д', ["..###", ".#..#", ".#..#", "#####", "#...#", "....."]),
    ('Е', ["#####", "#....", "#####", "#....", "#####", "...."]),
    ('Ж', ["#.#.#", "#.#.#", "#####", "#.#.#", "#.#.#", "...."]),
    ('З', ["#####", "....#", "#####", "....#", "#####", "...."]),
    ('И', ["#...#", "#...#", "##..#", "#.##.", "#...#", "...."]),
    ('Й', [".#.#.", "#...#", "#...#", "##..#", "#.##.", "#...#"]),
    ('К', ["#...#", "#..#.", "###..", "#..#.", "#...#", "...."]),
    ('Л', ["..###", ".#..#", "#...#", "#...#", "#...#", "...."]),
    ('М', ["#...#", "#####", "#.#.#", "#...#", "#...#", "...."]),
    ('Н', ["#...#", "#...#", "#####", "#...#", "#...#", "...."]),
    ('О', [".###.", "#...#", "#...#", "#...#", ".###.", "...."]),
    ('П', ["#####", "#...#", "#...#", "#...#", "#...#", "...."]),
    ('Р', ["#####", "#...#", "#####", "#....", "#....", "...."]),
    ('С', [".####", "#....", "#....", "#....", ".####", "...."]),
    ('Т', ["#####", "..#..", "..#..", "..#..", "..#..", "...."]),
    ('У', ["#...#", "#...#", ".####", "....#", "###..", "...."]),
    ('Ф', ["..#..", "#####", "#...#", "#####", "..#..", "...."]),
    ('Х', ["#...#", "#...#", "..#..", "#...#", "#...#", "...."]),
    ('Ц', ["#...#", "#...#", "#...#", "#####", "....#", "....#"]),
    ('Ч', ["#...#", "#...#", ".####", "....#", "....#", "...."]),
    ('Ш', ["#...#", "#...#", "#...#", "#...#", "#####", "...."]),
    ('Щ', ["#...#", "#...#", "#...#", "#...#", "#####", "....#"]),
    ('Ъ', ["##...", ".#...", ".####", "....#", "####.", "...."]),
    ('Ы', ["#...#", "####.", "#..#.", "####.", "....#", "...."]),
    ('Ь', ["#....", "#....", "####.", "#...#", "####.", "...."]),
    ('Э', ["####.", "#...#", "##..#", "#...#", "####.", "...."]),
    ('Ю', ["#.###", "#.#.#", "#####", "#.#.#", "#.###", "...."]),
    ('Я', ["####.", "#...#", "####.", "..#..", ".#...", "...."]),
    ('Ё', ["#.#.#", "#####", "#....", "#####", "#....", "#####"]),
    // --- Латиница (запасной) ---
    ('A', [".###.", "#...#", "#####", "#...#", "#...#", "...."]),
    ('B', ["#####", "#....", "#####", "#....", "#####", "...."]),
    ('C', [".####", "#....", "#....", "#....", ".####", "...."]),
    ('D', ["#####", "#...#", "#...#", "#...#", "#####", "...."]),
    ('E', ["#####", "#....", "#####", "#....", "#####", "...."]),
    ('F', ["#####", "#....", "#####", "#....", "#....", "...."]),
    ('G', [".####", "#....", "#..##", "#...#", ".####", "...."]),
    ('H', ["#...#", "#...#", "#####", "#...#", "#...#", "...."]),
    ('I', ["..#..", "..#..", "..#..", "..#..", "..#..", "...."]),
    ('J', ["....#", "....#", "....#", "#...#", ".###.", "...."]),
    ('K', ["#...#", "#..#.", "###..", "#..#.", "#...#", "...."]),
    ('L', ["#....", "#....", "#....", "#....", "#####", "...."]),
    ('M', ["#...#", "#####", "#####", "#...#", "#...#", "...."]),
    ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#", "...."]),
    ('O', [".###.", "#...#", "#...#", "#...#", ".###.", "...."]),
    ('P', ["#####", "#...#", "#####", "#....", "#....", "...."]),
    ('Q', [".###.", "#...#", "#...#", "#.##.", ".####", "...."]),
    ('R', ["#####", "#...#", "#####", "#..#.", "#...#", "...."]),
    ('S', [".####", "#....", ".###.", "....#", "####.", "...."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "...."]),
    ('U', ["#...#", "#...#", "#...#", "#...#", ".###.", "...."]),
    ('V', ["#...#", "#...#", "#...#", ".###.", "..#..", "...."]),
    ('W', ["#...#", "#...#", "#...#", "#####", "#...#", "...."]),
    ('X', ["#...#", "#...#", "..#..", "#...#", "#...#", "...."]),
    ('Y', ["#...#", "#...#", ".###.", "..#..", "..#..", "...."]),
    ('Z', ["#####", "....#", "..##.", ".#...", "#####", "...."]),
    // --- Цифры (запасной) ---
    ('0', [".###.", "#..##", "#.#.#", "##..#", ".###.", "...."]),
    ('1', ["..#..", ".##..", "..#..", "..#..", ".###.", "...."]),
    ('2', [".###.", "#...#", "..##.", ".#...", "#####", "...."]),
    ('3', ["####.", "....#", ".###.", "....#", "####.", "...."]),
    ('4', ["#...#", "#...#", "#####", "....#", "....#", "...."]),
    ('5', ["#####", "#....", "####.", "....#", "####.", "...."]),
    ('6', [".###.", "#....", "####.", "#...#", ".###.", "...."]),
    ('7', ["#####", "....#", "...#.", "..#..", "..#..", "...."]),
    ('8', [".###.", "#...#", ".###.", "#...#", ".###.", "...."]),
    ('9', [".###.", "#...#", ".####", "....#", ".###.", "...."]),
    // --- Пунктуация (запасной) ---
    ('.', [".....", ".....", ".....", ".....", "..#..", "...."]),
    (',', [".....", ".....", ".....", "..#..", ".#...", "...."]),
    ('!', ["..#..", "..#..", "..#..", ".....", "..#..", "...."]),
    ('?', [".###.", "#...#", "...#.", ".....", "..#..", "...."]),
    (':', [".....", "..#..", ".....", "..#..", ".....", "...."]),
    (';', [".....", "..#..", ".....", "..#..", ".#...", "...."]),
    ('-', [".....", ".....", "#####", ".....", ".....", "...."]),
    ('%', ["##..#", "##.#.", ".#...", "#.##.", "#..##", "...."]),
    ('(', ["...#.", "..#..", "..#..", "..#..", "...#.", "...."]),
    (')', [".#...", "..#..", "..#..", "..#..", ".#...", "...."]),
    ('/', ["....#", "...#.", "..#..", ".#...", "#....", "...."]),
    ('*', [".....", "#.#.#", ".###.", "#.#.#", ".....", "...."]),
    ('+', [".....", "..#..", "#####", "..#..", ".....", "...."]),
    ('=', [".....", "#####", ".....", "#####", ".....", "...."]),
    ('«', [".....", "#..#.", "#..#.", ".....", ".....", "...."]),
    ('»', [".....", ".#..#", ".#..#", ".....", ".....", "...."]),
    ('"', [".#.#.", ".#.#.", ".....", ".....", ".....", "...."]),
    (' ', [".....", ".....", ".....", ".....", ".....", "...."]),
];

/// Кегли можно переопределить переменной окружения (для отладки).
fn fontsize_env() -> Option<Vec<usize>> {
    std::env::var("KVASS_FONT_SIZES")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|p| p.trim().parse::<usize>().ok())
                .filter(|&p| (8..=200).contains(&p))
                .collect::<Vec<usize>>()
        })
        .filter(|v: &Vec<usize>| !v.is_empty())
}

impl Font {
    pub fn new() -> Self {
        let mut fallback = HashMap::new();
        for (ch, rows) in GLYPHS {
            let mut bits = [0u8; GLYPH_H];
            for (r, row) in rows.iter().enumerate() {
                let mut b = 0u8;
                for (c, ch2) in row.chars().enumerate() {
                    if ch2 == '#' {
                        b |= 1 << (GLYPH_W - 1 - c);
                    }
                }
                bits[r] = b;
            }
            fallback.insert(*ch, bits);
        }

        let want = fontsize_env().unwrap_or_else(|| SIZES.to_vec());
        let root = crate::assets_root();
        let candidates = [
            root.join("fonts").join("golos.ttf"),
            root.join("fonts").join("pixelify.ttf"),
            root.join("fonts").join("vt323.ttf"),
        ];
        let mut sizes: HashMap<usize, HashMap<char, Glyph>> = HashMap::new();
        let mut metrics: HashMap<usize, SizeMetrics> = HashMap::new();
        let mut ttf = false;
        let mut px_label = String::from("5x6");

        'load: for path in candidates {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Ok(fnt) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
                continue;
            };
            // кириллица обязательна — проба
            let probe = ['А', 'Б', 'К', 'м', 'ь'];
            let probe_px = *want.iter().max().unwrap_or(&30) as f32;
            let have = probe
                .iter()
                .filter(|&&c| fnt.rasterize(c, probe_px).0.width > 0)
                .count();
            eprintln!("font: {} — кириллица {have}/{}", path.display(), probe.len());
            if have < probe.len() {
                continue;
            }

            let chars: Vec<char> = {
                let mut v: Vec<char> = (' '..='~').collect();
                v.extend('А'..='Я');
                v.extend('а'..='я');
                v.extend(['Ё', 'ё', '№', '—', '–', '−', '«', '»', '…']);
                v
            };

            for &px in &want {
                let mut map: HashMap<char, Glyph> = HashMap::new();
                // капс — верх «Н» над базовой линией
                let (nm, _) = fnt.rasterize('Н', px as f32);
                let cap = nm.ymin + nm.height as i32;
                let mut desc = 0i32;
                for c in chars.clone() {
                    let (m, bmp) = fnt.rasterize(c, px as f32);
                    let adv = (m.advance_width.round() as i32).max(1);
                    if c == ' ' || m.width == 0 || m.height == 0 {
                        map.insert(
                            c,
                            Glyph {
                                adv,
                                xmin: 0,
                                ymin: 0,
                                w: 0,
                                h: 0,
                                cov: Vec::new(),
                            },
                        );
                        continue;
                    }
                    desc = desc.max(-(m.ymin));
                    let mut cov = vec![0u8; bmp.len()];
                    for (o, v) in cov.iter_mut().zip(bmp.iter()) {
                        // слабый шлейф сглаживания в ноль — фон остаётся чистым
                        *o = if *v < 26 { 0 } else { *v };
                    }
                    map.insert(
                        c,
                        Glyph {
                            adv,
                            xmin: m.xmin,
                            ymin: m.ymin,
                            w: m.width as usize,
                            h: m.height as usize,
                            cov,
                        },
                    );
                }
                let cap = cap.max(1);
                let desc = desc.max(0);
                let pad = (px as i32 / 7).max(3);
                let line = cap + desc + pad;
                // половина свободного места — над прописными, половина — под строкой
                let top_pad = ((line - cap - desc) / 2).max(0);
                sizes.insert(px, map);
                metrics.insert(px, (cap, desc, line, top_pad));
            }
            if let Some(m0) = sizes.get(&want[0]) {
                let missing: Vec<char> = chars
                    .iter()
                    .copied()
                    .filter(|&c| c != ' ')
                    .filter(|&c| raster_missing(m0, c))
                    .collect();
                if !missing.is_empty() {
                    eprintln!(
                        "font: {} — нет глифов ({}): {:?}",
                        path.display(),
                        missing.len(),
                        missing
                    );
                }
            }
            ttf = true;
            px_label = format!("{} px", want.iter().min().unwrap_or(&20));
            break 'load;
        }

        if !ttf {
            eprintln!("font: TTF не найдены — встроенный шрифт 5×6");
            for &px in &want {
                let sc = (px / 6).max(1) as i32;
                let cap = 5 * sc;
                let desc = sc;
                let line = cap + desc + 2;
                metrics.insert(px, (cap, desc, line, 0));
                sizes.insert(px, HashMap::new());
            }
            px_label = "5x6".into();
        }

        Self {
            fallback,
            sizes,
            metrics,
            ttf,
            px_label,
        }
    }

    fn metrics_of(&self, size: usize) -> SizeMetrics {
        let s = pick_nearest(size);
        *self
            .metrics
            .get(&s)
            .unwrap_or(&(s as i32, 0, s as i32 + 6, 0))
    }

    /// Высота строки (пиксели канваса).
    pub fn line(&self, size: usize) -> i32 {
        self.metrics_of(size).2
    }

    /// Высота прописных (для вертикального центрирования).
    pub fn cap_h(&self, size: usize) -> i32 {
        self.metrics_of(size).0
    }

    /// Ширина текста (пиксели канваса). Нет глифа — ширина «?».
    pub fn w(&self, text: &str, size: usize) -> i32 {
        let t: String = text.chars().map(norm_char).collect();
        if !self.ttf {
            let sc = (pick_nearest(size) as i32 / 6).max(1);
            return t.chars().count() as i32 * ((GLYPH_W + 1) as i32 * sc);
        }
        let s = pick_nearest(size);
        let map = match self.sizes.get(&s) {
            Some(m) if !m.is_empty() => m,
            _ => return t.chars().count() as i32 * s as i32,
        };
        let mut w = 0i32;
        for c in t.chars() {
            match map.get(&c) {
                Some(g) => w += g.adv.max(0),
                None => w += (GLYPH_W + 1) as i32 * (s as i32 / 6).max(1),
            }
        }
        w
    }

    /// Наибольший кегль из SIZES при котором текст влезает в max_w.
    pub fn fit(&self, text: &str, max_w: i32, max_size: usize) -> usize {
        let mut best = SIZES[0];
        for &s in SIZES.iter() {
            if s <= max_size && self.w(text, s) <= max_w {
                best = s;
            }
        }
        best
    }

    /// Рисует строку; `x`,`y` — верх строки.
    pub fn draw(
        &self,
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        text: &str,
        size: usize,
        col: [u8; 3],
        a: u8,
    ) {
        let s = pick_nearest(size);
        let t: String = text.chars().map(norm_char).collect();
        let (cap, _desc, _line, top_pad) = self.metrics_of(size);
        let baseline = y + top_pad + cap;
        let fallback_scale = (s as i32 / 6).max(1);
        if !self.ttf {
            // встроенный запасной 5×6
            let mut cx = x;
            for c in t.chars() {
                self.blit_fallback(canvas, &mut cx, baseline, size, &c, col, a);
            }
            return;
        }
        let map = match self.sizes.get(&s) {
            Some(m) if !m.is_empty() => m,
            _ => {
                let mut cx = x;
                for c in t.chars() {
                    self.blit_fallback(canvas, &mut cx, baseline, size, &c, col, a);
                }
                return;
            }
        };
        let mut cx = x;
        for c in t.chars() {
            match map.get(&c) {
                Some(g) if g.w > 0 && g.h > 0 => {
                    let gx = cx + g.xmin;
                    // верхняя строка растра: ymin+h-1 пикселей над базовой линией
                    let gy = baseline + 1 - g.ymin - g.h as i32;
                    self.blit_glyph(canvas, gx, gy, g, col, a);
                    cx += g.adv.max(0);
                }
                // пустой глиф (пробел и т.п.) — только шаг каретки
                Some(g) => cx += g.adv.max(0),
                None => {
                    self.blit_fallback(canvas, &mut cx, baseline, size, &c, col, a);
                }
            }
        }
    }

    /// Строка с мягкой тенью (+2 px вниз) — читаемость на любом фоне.
    pub fn draw_sh(
        &self,
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        text: &str,
        size: usize,
        col: [u8; 3],
        a: u8,
    ) {
        self.draw(
            canvas,
            x,
            y + 2,
            text,
            size,
            [0, 0, 0],
            ((a as u32 * 3) / 4).min(255) as u8,
        );
        self.draw(canvas, x, y, text, size, col, a);
    }

    /// Вписывает и центрирует текст в прямоугольнике. Возвращает высоту строки.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_fit_sh(
        &self,
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        w: i32,
        text: &str,
        max_size: usize,
        col: [u8; 3],
        a: u8,
    ) -> i32 {
        let s0 = pick(max_size);
        let s = if self.w(text, s0) <= w {
            s0
        } else {
            self.fit(text, w, s0)
        };
        let tx = x + ((w - self.w(text, s)) / 2).max(0);
        self.draw_sh(canvas, tx, y, text, s, col, a);
        self.line(s)
    }

    /// Вписывает и рисует слева. Возвращает высоту строки.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_fit(
        &self,
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        w: i32,
        text: &str,
        max_size: usize,
        col: [u8; 3],
        a: u8,
    ) -> i32 {
        let s0 = pick(max_size);
        let s = if self.w(text, s0) <= w {
            s0
        } else {
            self.fit(text, w, s0)
        };
        self.draw_sh(canvas, x, y, text, s, col, a);
        self.line(s)
    }

    fn blit_glyph(&self, canvas: &mut Canvas, x: i32, y: i32, g: &Glyph, col: [u8; 3], a: u8) {
        if a == 0 {
            return;
        }
        for my in 0..g.h as i32 {
            for mx in 0..g.w as i32 {
                let v = g.cov[my as usize * g.w + mx as usize];
                if v == 0 {
                    continue;
                }
                let alpha = ((a as u32 * v as u32) / 255).min(255) as u8;
                canvas.blend(x + mx, y + my, col, alpha);
            }
        }
    }

    fn blit_fallback(
        &self,
        canvas: &mut Canvas,
        cx: &mut i32,
        baseline: i32,
        size: usize,
        ch: &char,
        col: [u8; 3],
        a: u8,
    ) {
        let scale = (size as i32 / 6).max(1);
        let bits = match self.fallback.get(ch).or_else(|| self.fallback.get(&'?')) {
            Some(b) => b,
            None => {
                *cx += (GLYPH_W as i32 + 1) * scale;
                return;
            }
        };
        // верх прописных на базовой линии, как у TTF (капс 5 строк)
        let top = baseline - 5 * scale;
        for r in 0..GLYPH_H as i32 {
            for c in 0..GLYPH_W as i32 {
                let on = bits[r as usize] & (1 << (GLYPH_W as i32 - 1 - c)) != 0;
                if !on {
                    continue;
                }
                let x = *cx + c * scale;
                let y = top + r * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        canvas.blend(x + dx, y + dy, col, a);
                    }
                }
            }
        }
        *cx += (GLYPH_W as i32 + 1) * scale;
    }
}

