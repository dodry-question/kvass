// Проверка метрик fontdue: ymin — это верх или низ растра относительно базовой линии?
use fontdue::Font;

fn main() {
    let bytes = std::fs::read("assets/fonts/golos.ttf").expect("golos.ttf");
    let f = Font::from_bytes(bytes, fontdue::FontSettings::default()).expect("parse");
    for ch in ['Н', 'р', 'о', 'й', 'Ё', 'ц', 'ф', 'A', 'g'] {
        let (m, bmp) = f.rasterize(ch, 30.0);
        let mut first = None;
        let mut last = None;
        for (i, v) in bmp.iter().enumerate() {
            if *v > 64 {
                let r = i / m.width.max(1);
                if first.is_none() {
                    first = Some(r);
                }
                last = Some(r);
            }
        }
        println!(
            "{ch}: xmin={} ymin={} w={} h={} adv={:.1} ink_rows={first:?}..{last:?}",
            m.xmin, m.ymin, m.width, m.height, m.advance_width
        );
    }
}
