//! Текст в палитровом буфере 320x200.
//! Основной шрифт — пиксельный TTF (assets/fonts/vt323.ttf), растеризуется
//! fontdue в бинарные маски при старте. Если TTF нет — запасной встроенный
//! растровый шрифт 5x6 (только заглавные).

use std::collections::HashMap;

use crate::raycaster::UI_SCALE;

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

/// Есть ли глиф в растровой карте (непустой).
fn raster_missing(r: &HashMap<char, GlyphRaster>, c: char) -> bool {
    r.get(&c).map_or(true, |g| g.w == 0)
}

/// Растеризованный глиф TTF: бинарная маска + шаг.
struct GlyphRaster {
    adv: i32,
    w: usize,
    h: usize,
    /// Смещение первой строки маски от строки базовой линии
    /// (вверх от базовой линии — отрицательное). Общая для всех
    /// глифов сетка — буквы одного кегля не «прыгают» по высоте.
    top: i32,
    /// Левый отступ чернил от позиции каретки.
    xmin: i32,
    mask: Vec<u8>,
}

/// Маска глифа на сетке, привязанной к базовой линии: строка j покрывает
/// высотомерные пиксели [j*SS, (j+1)*SS) относительно базовой линии
/// (div_euclid, т.е. строки над линией — отрицательные). Раньше маска
/// строилась как div_ceil(h_hi, SS) без привязки — каждый глиф садился
/// на сетку со случайным сдвигом до 1px, из-за чего «О» была выше «Н»,
/// а тонкие штрихи «Н» рвались. Возвращает (jmin, w, h, mask).
fn glyph_mask(
    m: &fontdue::Metrics,
    bitmap: &[u8],
    th: u8,
) -> Option<(i32, usize, usize, Vec<u8>)> {
    if m.width == 0 || m.height == 0 {
        return None;
    }
    let ss = TTF_SS as i32;
    let w_hi = m.width as i32;
    let h_hi = m.height as i32;
    let y0 = m.ymin;
    let jmin = y0.div_euclid(ss);
    let jmax = (y0 + h_hi - 1).div_euclid(ss);
    let h = (jmax - jmin + 1) as usize;
    let w = (m.width as usize).div_ceil(TTF_SS);
    let mut mask = vec![0u8; w * h];
    for j in 0..h {
        for i in 0..w {
            let mut sum = 0u32;
            let mut n = 0u32;
            let r0 = (jmin + j as i32) * ss;
            for r in r0..r0 + ss {
                if r < y0 || r >= y0 + h_hi {
                    continue;
                }
                let br = (r - y0) as usize;
                for c in (i as i32 * ss)..((i as i32 + 1) * ss).min(w_hi) {
                    sum += bitmap[br * w_hi as usize + c as usize] as u32;
                    n += 1;
                }
            }
            if n > 0 && sum / n >= th as u32 {
                mask[j * w + i] = 1;
            }
        }
    }
    // Одиночные точки-шум от сглаживания убираем только когда растр
    // строился через суперсэмплинг (TTF_SS > 1). При чистой родной
    // сетке шрифта одиночный пиксель — не шум, а настоящая точка
    // диакритики («Ё», «й»), трогать её нельзя.
    let mut clean = mask.clone();
    if TTF_SS > 1 {
        for j in 0..h {
            for i in 0..w {
                if mask[j * w + i] == 0 {
                    continue;
                }
                let mut nb = 0;
                for dj in -1i32..=1 {
                    for di in -1i32..=1 {
                        if dj == 0 && di == 0 {
                            continue;
                        }
                        let (jj, ii) = (j as i32 + dj, i as i32 + di);
                        if jj >= 0
                            && jj < h as i32
                            && ii >= 0
                            && ii < w as i32
                            && mask[jj as usize * w + ii as usize] == 1
                        {
                            nb += 1;
                        }
                    }
                }
                if nb == 0 {
                    clean[j * w + i] = 0;
                }
            }
        }
    }
    Some((jmin, w, h, clean))
}

/// Размер растеризации TTF (пиксели).
///
/// Важно: pixelify.ttf нарисован автором на сетке 10px. Растеризация
/// строго в кратном размере (10, 20, 30...) даёт 100% чистую двоичную
/// сетку — буквы ровно такие, как их задумал шрифтовой художник.
/// Промежуточные размеры (12, 24...) дают ~78% сглаживания, а порог
/// потом режет штрихи рвано — именно это и было видно как «рваные»
/// буквы. 20px = 2× родной сетки: капслок ~7px в дизайновых координатах
/// (320×200), т.е. прежний кегль, но без мыла.
const TTF_PX: f32 = 20.0;
/// Суперсэмплинг выключен (1): шрифт уже пиксельный, усреднение только
/// ломает его сетку. Маска строится в TTF_PX и бинаризуется порогом.
const TTF_SS: usize = 1;
/// Порог сглаживания -> чёрный пиксель. При чистой родной сетке работает
/// как «не белое — значит чернило», остатки сглаживания убирает чистка
/// одиночных пикселей в glyph_mask.
const TTF_THRESHOLD: u8 = 96;

/// Размер растеризации: можно переопределить KVASS_FONT_PX (для подбора).
fn ttf_px() -> f32 {
    std::env::var("KVASS_FONT_PX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TTF_PX)
}

/// Порог бинаризации: KVASS_FONT_THRESHOLD.
fn ttf_threshold() -> u8 {
    std::env::var("KVASS_FONT_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(TTF_THRESHOLD)
}

pub struct Font {
    fallback: HashMap<char, [u8; GLYPH_H]>,
    raster: HashMap<char, GlyphRaster>,
    /// Высота прописных в строках маски (кегль). Общая для всех букв —
    /// «О» и «Н» одного размера.
    cap_h: i32,
    /// На сколько строк выносные элементы (у, р, д, ц, щ, ф) уходят
    /// ниже базовой линии. Нужно, чтобы строки не наезжали друг на друга.
    descent: i32,
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

        let mut raster = HashMap::new();
        // метрики кегля (заполняются при успешной загрузке TTF)
        let mut out_cap_h = (GLYPH_H - 1) as i32;
        let mut out_descent = 0i32;
        let px = ttf_px();
        let th = ttf_threshold();
        let root = crate::assets_root();
        let candidates = [root.join("fonts").join("vt323.ttf"), root.join("fonts").join("pixelify.ttf")];
        'load: for path in candidates {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Ok(fnt) = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) else {
                continue;
            };
            // кириллица обязательна — проба
            let probe = ['А', 'Б', 'К', 'м', 'ь'];
            let have = probe
                .iter()
                .filter(|&&c| fnt.rasterize(c, px).0.width > 0)
                .count();
            eprintln!("font: {} — кириллица {have}/{}", path.display(), probe.len());
            if have < probe.len() {
                continue;
            }
            let mut chars: Vec<char> = (' '..='~').collect();
            chars.extend('А'..='Я');
            chars.extend('а'..='я');
            chars.extend(['Ё', 'ё', '№', '—', '«', '»', '…']);
            let ss = TTF_SS as i32;
            let px_hi = px * TTF_SS as f32;
            // базовая линия — по строке, на которой стоит «Н»
            let (nm, _) = fnt.rasterize('Н', px_hi);
            let cap_top = nm.ymin.div_euclid(ss);
            let cap_h = ((nm.ymin + nm.height as i32 - 1).div_euclid(ss) - cap_top + 1).max(1);
            let base_row = cap_top + cap_h - 1;
            let mut descent = 0i32;
            for c in chars.clone() {
                let (m, bitmap) = fnt.rasterize(c, px_hi);
                if c == ' ' || m.width == 0 || m.height == 0 {
                    // пустой глиф (пробел) — только шаг
                    let adv = ((m.advance_width / TTF_SS as f32).round() as i32).max(1);
                    raster.insert(
                        c,
                        GlyphRaster {
                            adv,
                            w: 0,
                            h: 0,
                            top: 0,
                            xmin: 0,
                            mask: Vec::new(),
                        },
                    );
                    continue;
                }
                // маска на общей сетке базовой линии — буквы одного кегля
                // не «прыгают» по высоте (раньше «О» выходила выше «Н»)
                let Some((jmin, w, h, mask)) = glyph_mask(&m, &bitmap, th) else {
                    continue;
                };
                // Подрезаем «округлую» вылетку до общего бокса прописных.
                // В TTF круглые буквы (О, С, ...) на 1px выше/ниже линий —
                // в пиксельном кегле это читается как «буквы разного
                // размера». Диакритику (Ё, Й) не трогаем: над прописной
                // у неё пустая строка-разрыв, т.е. нет «прирастания».
                let mut jmin = jmin;
                let mut h = h as i32;
                let mut mask = mask;
                let mut trimmed = 0i32;
                while trimmed < 2 && jmin < cap_top && h > cap_h && w >= 2 {
                    let top_ok = mask[0] == 1 && mask[w] == 1;
                    if !top_ok {
                        break;
                    }
                    mask.drain(0..w);
                    jmin += 1;
                    h -= 1;
                    trimmed += 1;
                }
                while trimmed < 2 && jmin + h - 1 > base_row && h > cap_h && w >= 2 {
                    let last = (h - 1) as usize * w;
                    let prev = (h - 2) as usize * w;
                    let bot_ok = mask[last] == 1 && mask[prev] == 1;
                    if !bot_ok {
                        break;
                    }
                    mask.truncate(last);
                    h -= 1;
                    trimmed += 1;
                }
                descent = descent.max(jmin + h - 1 - base_row);
                // шаг глифа не меньше реальных чернил — буквы не наезжают
                let adv_hi = (m.advance_width.round() as i32).max(1);
                let adv = ((adv_hi as f32 / TTF_SS as f32).round() as i32)
                    .max(1)
                    .max(w as i32);
                raster.insert(
                    c,
                    GlyphRaster {
                        adv,
                        w,
                        h: h as usize,
                        top: jmin - cap_top,
                        xmin: m.xmin.div_euclid(ss),
                        mask,
                    },
                );
            }
            out_cap_h = cap_h;
            out_descent = descent.max(0);
            // диагностика: какие символы шрифт не содержит
            let missing: Vec<char> = chars
                .iter()
                .copied()
                .filter(|&c| c != ' ')
                .filter(|&c| raster_missing(&raster, c))
                .collect();
            if !missing.is_empty() {
                eprintln!(
                    "font: {} — нет глифов ({}): {:?}",
                    path.display(),
                    missing.len(),
                    missing
                );
            }
            break 'load;
        }

        Self {
            fallback,
            raster,
            cap_h: out_cap_h,
            descent: out_descent,
        }
    }

    /// Высота прописных (кегль) в строках маски — базовая единица.
    pub fn cap_rows(&self) -> i32 {
        self.cap_h.max(1)
    }

    /// Полная высота строки текста при заданном целочисленном масштабе,
    /// в дизайновых пикселях: кегль + выносные + воздух под тень.
    /// Ровно столько, сколько нужно строке — ничего не наезжает.
    pub fn line_at(&self, scale: usize) -> i32 {
        let s = scale.max(1) as i32;
        ((self.cap_h + self.descent) * s + s + UI_SCALE - 1) / UI_SCALE
    }

    /// Ширина текста при заданном масштабе, в дизайновых пикселях.
    pub fn width_at(&self, text: &str, scale: usize) -> i32 {
        let s = scale.max(1) as i32;
        if !self.raster.is_empty() {
            (text.chars().map(|c| self.advance(c).max(0)).sum::<i32>().max(0) * s
                + UI_SCALE
                - 1)
                / UI_SCALE
        } else {
            ((text.chars().count() * (GLYPH_W + 1)) as i32 * s + UI_SCALE - 1) / UI_SCALE
        }
    }

    /// Подбирает наибольший масштаб (от 1 до max_scale), при котором текст
    /// влезает в max_w (дизайновых px). Если и с масштабом 1 не влезает —
    /// возвращает 1 (вызывающий сам переносит или обрезает осознанно).
    pub fn fit_scale(&self, text: &str, max_w: i32, max_scale: usize) -> usize {
        for s in (1..=max_scale.max(1)).rev() {
            if self.width_at(text, s) <= max_w {
                return s;
            }
        }
        1
    }

    /// Рисует текст в прямоугольнике [x, x+w): подбирает масштаб под ширину
    /// и центрирует. Не выходит за рамки и не режется. Возвращает высоту
    /// строки в дизайновых px — чтобы вызывающий мог_STACKовать строки.
    pub fn draw_fit(
        &self,
        buf: &mut [u8],
        bw: usize,
        bh: usize,
        x: i32,
        y: i32,
        w: i32,
        text: &str,
        color: u8,
        outline: Option<u8>,
        max_scale: usize,
    ) -> i32 {
        let s = self.fit_scale(text, w, max_scale);
        let tw = self.width_at(text, s);
        let tx = x + ((w - tw) / 2).max(0);
        self.draw_scaled(buf, bw, bh, tx, y, text, color, outline, s);
        self.line_at(s)
    }

    fn glyph(&self, ch: char) -> Option<&[u8; GLYPH_H]> {
        let ch = match ch {
            '—' | '–' | '−' => '-',
            '…' => '.',
            _ => ch,
        };
        if self.raster.is_empty() {
            let up: char = ch.to_uppercase().next().unwrap_or(ch);
            return self.fallback.get(&up).or_else(|| self.fallback.get(&'?'));
        }
        self.fallback.get(&ch).or_else(|| self.fallback.get(&'?'))
    }

    fn advance(&self, ch: char) -> i32 {
        if let Some(g) = self.raster.get(&ch) {
            g.adv
        } else {
            (GLYPH_W + 1) as i32
        }
    }

    /// Ширина текста в дизайновых пикселях при масштабе 1.
    pub fn width(&self, text: &str) -> usize {
        self.width_at(text, 1).max(0) as usize
    }

    /// Рисует текст с обрезкой по краям. `outline` — цвет тени (+1/+1).
    pub fn draw(
        &self,
        buf: &mut [u8],
        bw: usize,
        bh: usize,
        x: i32,
        y: i32,
        text: &str,
        color: u8,
        outline: Option<u8>,
    ) {
        self.draw_scaled(buf, bw, bh, x, y, text, color, outline, 1);
    }

    /// То же с целочисленным масштабом. `x`, `y` — дизайновые координаты
    /// (пространство 320×200); реальные = ×UI_SCALE. Растр TTF уже в
    /// реальных px (24px), поэтому рисуется 1:1 при scale=1.
    pub fn draw_scaled(
        &self,
        buf: &mut [u8],
        bw: usize,
        bh: usize,
        x: i32,
        y: i32,
        text: &str,
        color: u8,
        outline: Option<u8>,
        scale: usize,
    ) {
        let s = scale as i32;
        let base = s * UI_SCALE; // масштаб запасного 5×6 шрифта (дизайновый)
        let mut cx = x * UI_SCALE;
        let ry = y * UI_SCALE;
        for ch in text.chars() {
            if !self.raster.is_empty() {
                if let Some(g) = self.raster.get(&ch) {
                    if g.w > 0 {
                        let gy = ry + g.top * s;
                        let gx = cx + g.xmin * s;
                        if let Some(o) = outline {
                            let off = s * UI_SCALE;
                            self.blit_mask(buf, bw, bh, gx + off, gy + off, g, s, o);
                        }
                        self.blit_mask(buf, bw, bh, gx, gy, g, s, color);
                    }
                    cx += g.adv * s;
                    continue;
                }
                // символа нет в TTF — запасная растровая таблица
                let norm = match ch {
                    '—' | '–' | '−' => '-',
                    '…' => '.',
                    _ => ch,
                };
                let up: char = norm.to_uppercase().next().unwrap_or(norm);
                let Some(bits) = self.fallback.get(&up).or_else(|| self.fallback.get(&'?')) else {
                    cx += ((GLYPH_W + 1) as i32) * base;
                    continue;
                };
                self.blit_fallback(buf, bw, bh, &mut cx, ry, bits, base, color, outline);
                continue;
            }
            let Some(bits) = self.glyph(ch) else {
                cx += ((GLYPH_W + 1) as i32) * base;
                continue;
            };
            self.blit_fallback(buf, bw, bh, &mut cx, ry, bits, base, color, outline);
        }
    }

    fn blit_mask(
        &self,
        buf: &mut [u8],
        bw: usize,
        bh: usize,
        x: i32,
        y: i32,
        g: &GlyphRaster,
        s: i32,
        color: u8,
    ) {
        for my in 0..g.h {
            for mx in 0..g.w {
                if g.mask[my * g.w + mx] == 0 {
                    continue;
                }
                for sy in 0..s {
                    for sx in 0..s {
                        put_raw(buf, bw, bh, x + mx as i32 * s + sx, y + my as i32 * s + sy, color);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn blit_fallback(
        &self,
        buf: &mut [u8],
        bw: usize,
        bh: usize,
        cx: &mut i32,
        y: i32,
        bits: &[u8; GLYPH_H],
        s: i32,
        color: u8,
        outline: Option<u8>,
    ) {
        if let Some(o) = outline {
            for (r, row) in bits.iter().enumerate() {
                for c in 0..GLYPH_W {
                    if row & (1 << (GLYPH_W - 1 - c)) != 0 {
                        for sy in 0..s {
                            for sx in 0..s {
                                let px = *cx + c as i32 * s + sx + s;
                                let py = y + r as i32 * s + sy + s;
                                put_raw(buf, bw, bh, px, py, o);
                            }
                        }
                    }
                }
            }
        }
        for (r, row) in bits.iter().enumerate() {
            for c in 0..GLYPH_W {
                if row & (1 << (GLYPH_W - 1 - c)) != 0 {
                    for sy in 0..s {
                        for sx in 0..s {
                            let px = *cx + c as i32 * s + sx;
                            let py = y + r as i32 * s + sy;
                            put_raw(buf, bw, bh, px, py, color);
                        }
                    }
                }
            }
        }
        *cx += ((GLYPH_W + 1) as i32) * s;
    }
}

/// Пиксель в реальных координатах буфера (без масштаба) — для внутренних
/// блитов шрифта.
fn put_raw(buf: &mut [u8], bw: usize, bh: usize, x: i32, y: i32, c: u8) {
    if x >= 0 && y >= 0 && (x as usize) < bw && (y as usize) < bh {
        buf[y as usize * bw + x as usize] = c;
    }
}

/// Пиксель UI в дизайновых координатах: рисуется блоком UI_SCALE×UI_SCALE,
/// чтобы панели/иконки/облачка выглядели так же, но вдвое крупнее.
pub fn put(buf: &mut [u8], bw: usize, bh: usize, x: i32, y: i32, c: u8) {
    let (rx, ry) = (x * UI_SCALE, y * UI_SCALE);
    for dy in 0..UI_SCALE {
        for dx in 0..UI_SCALE {
            put_raw(buf, bw, bh, rx + dx, ry + dy, c);
        }
    }
}

pub fn rect(buf: &mut [u8], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u8) {
    let (rx, ry, rw, rh) = (x * UI_SCALE, y * UI_SCALE, w * UI_SCALE, h * UI_SCALE);
    for yy in ry..ry + rh {
        for xx in rx..rx + rw {
            put_raw(buf, bw, bh, xx, yy, c);
        }
    }
}

pub fn frame_rect(buf: &mut [u8], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u8) {
    let (rx, ry, rw, rh) = (x * UI_SCALE, y * UI_SCALE, w * UI_SCALE, h * UI_SCALE);
    for xx in rx..rx + rw {
        put_raw(buf, bw, bh, xx, ry, c);
        put_raw(buf, bw, bh, xx, ry + rh - 1, c);
    }
    for yy in ry..ry + rh {
        put_raw(buf, bw, bh, rx, yy, c);
        put_raw(buf, bw, bh, rx + rw - 1, yy, c);
    }
}
