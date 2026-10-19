//! Оффскрин-самотест: рисует кадры всех экранов в BMP-файлы и выходит.
//! Запуск: `cargo run -- --selftest`. Файлы в `selftest_out/`.

use std::fs;
use std::path::Path;

use bevy::prelude::Vec2;

use crate::brewing::{BarrelPhase, Barrels, BrewQte};
use crate::day_cycle::{GameState, Phase};
use crate::font::{rect, Font};
use crate::inventory::{HandItem, Inventory};
use crate::map::Map;
use crate::palette::idx;
use crate::raycaster::{GameCam, Renderer, SCREEN_H, SCREEN_W, UI_H, UI_W};
use crate::sprites::{NpcKind, SpriteSheet, Vessel};
use crate::textures::Textures;
use crate::ui;

pub fn run() {
    let out = Path::new("selftest_out");
    fs::create_dir_all(out).expect("создать selftest_out");
    let map = Map::shop();
    let textures = Textures::generate();
    let sprites = SpriteSheet::generate();
    let font = Font::new();
    // диагностика кегля: высота строки и ширина ключевых строк
    for s in 1..=4 {
        eprintln!(
            "font metrics: scale={s} line_at={} ширина[КВАСНАЯ]={} ЛАВКА={} JAM={}",
            font.line_at(s),
            font.width_at("КВАСНАЯ", s),
            font.width_at("ЛАВКА", s),
            font.width_at("VYATKA OWLS GAME JAM", s),
        );
    }
    eprintln!(
        "font metrics: fit_scale(КВАСНАЯ, 312, 4)={} fit_scale(JAM, 312, 1)={}",
        font.fit_scale("КВАСНАЯ", UI_W - 8, 4),
        font.fit_scale("VYATKA OWLS GAME JAM", UI_W - 8, 1),
    );
    let mut renderer = Renderer::new();
    let cam = GameCam {
        x: 5.5,
        y: 4.8,
        angle: -std::f32::consts::FRAC_PI_2 - 0.4,
    };
    let empty = ui::CustomerView { list: &[] };
    let no_qte = BrewQte::default();

    // --- 1. Главное меню ---
    let state = GameState {
        phase: Phase::Menu,
        ..Default::default()
    };
    ui::draw_frame(
        &mut renderer,
        &font,
        &state,
        &Inventory::default(),
        &Barrels::from_map(&map),
        &no_qte,
        &empty,
        1.0,
        &cam,
        &map,
    );
    renderer.present();
    write_bmp(&out.join("1_menu.bmp"), &renderer.rgba);

    // --- 2. Склад (закупка) ---
    let inv = Inventory {
        hands: [None; 4],
        selected: 0,
        stock_croutons: 2,
        stock_sugar: 2,
        stock_glass: 4,
        stock_jar: 2,
        stock_bottle: 1,
    };
    let state = GameState {
        phase: Phase::Shopping,
        msg: "НАЧАЛО ДНЯ 1. ЗАКУПИТЕ СЫРЬЁ (1-3), ЗАТЕМ ENTER".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    ui::draw_frame(
        &mut renderer,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &empty,
        2.0,
        &cam,
        &map,
    );
    renderer.present();
    write_bmp(&out.join("2_shopping.bmp"), &renderer.rgba);

    // --- 3. Игровой день: мир + HUD + диалоги ---
    let mut barrels = Barrels::from_map(&map);
    let mut keys: Vec<(i32, i32)> = barrels.map.keys().copied().collect();
    keys.sort();
    if let Some(b) = barrels.map.get_mut(&keys[0]) {
        b.phase = BarrelPhase::Empty;
        b.step = 0;
    }
    if let Some(b) = barrels.map.get_mut(&keys[1]) {
        b.phase = BarrelPhase::Ferment(42.0);
        b.step = 2;
    }
    if let (Some(b0), Some(b1)) = (keys.get(0), keys.get(1)) {
        if b0 != b1 {
            // две бочки с разными фазами — как советует игра
            if let Some(b) = barrels.map.get_mut(b1) {
                b.phase = BarrelPhase::Ready(104.0);
                b.portions = 13;
            }
        }
    }
    let mut inv = Inventory {
        hands: [
            Some(HandItem::Croutons(2)),
            Some(HandItem::Full(Vessel::Glass, 1, false)),
            Some(HandItem::Full(Vessel::Bottle, 1, true)),
            Some(HandItem::Bucket),
        ],
        selected: 1,
        stock_croutons: 2,
        stock_sugar: 2,
        stock_glass: 4,
        stock_jar: 2,
        stock_bottle: 1,
    };
    let state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 47,
        norm: 140,
        day_earned: 60,
        time_left: 137.0,
        msg: "СУХАРИ КОНЧИЛИСЬ — КУПИТЕ (1) НА СКЛАДЕ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let renders = vec![
        ui::CustomerRender {
            kind: NpcKind::Grandma,
            pos: Vec2::new(3.5, 2.7),
            bubble: Some("Дай, внучек, стаканчик квасика, холодненький...".into()),
        },
        ui::CustomerRender {
            kind: NpcKind::Worker,
            pos: Vec2::new(5.5, 2.7),
            bubble: Some("Стакан, но погуще, а то опять водица.".into()),
        },
    ];
    let view = ui::CustomerView { list: &renders };
    renderer.render(&map, &textures, &cam, 1.0);
    let list: Vec<(Vec2, NpcKind)> = renders.iter().map(|r| (r.pos, r.kind)).collect();
    renderer.draw_sprites(&sprites, &list, &cam, 1.0);
    ui::draw_frame(&mut renderer, &font, &state, &inv, &barrels, &no_qte, &view, 3.5, &cam, &map);
    renderer.present();
    write_bmp(&out.join("3_playing.bmp"), &renderer.rgba);

    // --- 4. Итоги дня ---
    let state = GameState {
        phase: Phase::DayEnd,
        day: 2,
        money: 147,
        norm: 140,
        day_earned: 117,
        fine: 30,
        day_passed: true,
        ..Default::default()
    };
    ui::draw_frame(
        &mut renderer,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &empty,
        4.0,
        &cam,
        &map,
    );
    renderer.present();
    write_bmp(&out.join("4_dayend.bmp"), &renderer.rgba);

    // --- 5. Финал ---
    let state = GameState {
        phase: Phase::Final,
        ..Default::default()
    };
    ui::draw_frame(
        &mut renderer,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &empty,
        5.0,
        &cam,
        &map,
    );
    renderer.present();
    write_bmp(&out.join("5_final.bmp"), &renderer.rgba);

    // --- 6. Шрифт: все глифы крупно + образцы реальных строк ---
    // Раскладка по метрикам: строки складываются по измеренной высоте,
    // длинные подбирают масштаб под ширину — ничего не наезжает.
    let buf = &mut renderer.indices;
    buf.fill(idx::SHADOW);
    let rows = [
        "АБВГДЕЁЖЗИЙ",
        "КЛМНОПРСТУ",
        "ФХЦЧШЩЪЫЬЭЮЯ",
        "ABCDEFGHIJKLM",
        "NOPQRSTUVWXYZ",
        "0123456789 !?",
        "КАВС ДО КОНЦА",
    ];
    // кегль алфавита: наибольший, при котором все строки влезают в бюджет
    let alpha_budget = 118i32;
    let mut alpha_scale = 1usize;
    for s in (1..=2usize).rev() {
        let total: i32 = rows.iter().map(|r| font.line_at(font.fit_scale(r, UI_W - 8, s))).sum();
        if total <= alpha_budget {
            alpha_scale = s;
            break;
        }
    }
    let mut ry = 2i32;
    for r in rows.iter() {
        let s = font.fit_scale(r, UI_W - 8, alpha_scale);
        font.draw_scaled(buf, SCREEN_W, SCREEN_H, 4, ry, r, idx::WHITE, Some(idx::BLACK), s);
        ry += font.line_at(s);
    }
    println!("font: алфабет (кегль {alpha_scale}) занял {ry} px, бюджет {alpha_budget}");
    // диагностика: самые длинные строки не должны вылезать за 320
    for s in [
        "БОЧКА: E - РЕЦЕПТ, ДЕРЖИТЕ E - РОЗЛИВ",
        "ШАГ 4/7. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        "НАЧАЛО: СУХАРИ 2, САХАР 2, ПОСУДА 4+4+1",
        "СКЛАД 14 14",
        "ПОС 20 20 17",
        "ENTER - НАЧАТЬ (ОБУЧЕНИЕ)",
        "Дай, внучек, стаканчик квасика, холодненький...",
    ] {
        println!("font.width({s:?}) = {}", font.width(s));
    }
    // образцы реальных строк (масштаб1, как в игре)
    let samples = [
        "ПРИЛАВОК: E - ОТДАТЬ ПОКУПАТЕЛЮ",
        "Б2 ГОТОВА 13/20 12",
        "ШАГ 4/7. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        "Дай, внучек, стаканчик квасика...",
        "НАЧАЛО. СУХАРИ 2, САХАР 2, ПОСУДА 4+4+1",
    ];
    let sy0 = ry.max(124);
    let sy1 = sy0 + samples.len() as i32 * font.line_at(1);
    rect(buf, SCREEN_W, SCREEN_H, 0, sy0, UI_W, sy1 - sy0, idx::SHADOW);
    let mut sy = sy0;
    for r in samples.iter() {
        font.draw(buf, SCREEN_W, SCREEN_H, 3, sy, r, idx::LAMP, Some(idx::BLACK));
        sy += font.line_at(1);
    }
    let ty = (sy + 2).max(0).min(UI_H - 14);
    rect(buf, SCREEN_W, SCREEN_H, 0, ty, UI_W, (UI_H - ty).max(1), idx::CEILING);
    font.draw_fit(buf, SCREEN_W, SCREEN_H, 4, ty, UI_W - 8, "КВАСНАЯ ЛАВКА", idx::LAMP, Some(idx::BLACK), 2);
    renderer.present();
    write_bmp(&out.join("6_font.bmp"), &renderer.rgba);

    // --- 7. Обучение: панель инструкции + мигающий маркер на бочку ---
    let state = GameState {
        phase: Phase::Tutorial,
        tut_step: 6,
        msg: "ТАК ДЕРЖАТЬ!".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let tut_renders = vec![ui::CustomerRender {
        kind: NpcKind::Grandma,
        pos: Vec2::new(3.5, 2.7),
        bubble: None,
    }];
    let tut_view = ui::CustomerView { list: &tut_renders };
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(3.5, 2.7), NpcKind::Grandma)], &cam, 1.0);
    ui::draw_frame(&mut renderer, &font, &state, &inv, &barrels, &no_qte, &tut_view, 1.5, &cam, &map);
    renderer.present();
    write_bmp(&out.join("7_tutorial.bmp"), &renderer.rgba);

    // --- 8. Лист персонажей (проверка образов NPC) ---
    {
        let kinds = [
            NpcKind::Grandma,
            NpcKind::Worker,
            NpcKind::Mama,
            NpcKind::Student,
            NpcKind::Militsioner,
            NpcKind::Hooligan,
            NpcKind::Sanepid,
        ];
        let buf = &mut renderer.indices;
        buf.fill(idx::CEILING);
        for (i, k) in kinds.iter().enumerate() {
            let spr = sprites.npc_sprite(*k);
            let ox = 6 + i as i32 * 44;
            for y in 0..64 {
                for x in 0..32 {
                    let c = spr[y * 32 + x];
                    if c == crate::sprites::TRANSPARENT {
                        continue;
                    }
                    for dy in 0..2 {
                        for dx in 0..2 {
                            rect(buf, SCREEN_W, SCREEN_H, ox + x as i32 * 2 + dx, 20 + y as i32 * 2 + dy, 1, 1, c);
                        }
                    }
                }
            }
            font.draw(buf, SCREEN_W, SCREEN_H, ox, 152, k.name(), idx::LAMP, Some(idx::BLACK));
        }
        font.draw(buf, SCREEN_W, SCREEN_H, 4, 170, "БАБУШКА РАБОЧИЙ МАМА СТУДЕНТ МИЛИЦИОНЕР", idx::WHITE, Some(idx::BLACK));
        font.draw(buf, SCREEN_W, SCREEN_H, 4, 180, "ХУЛИГАН САНЭПИД", idx::WHITE, Some(idx::BLACK));
        renderer.present();
        write_bmp(&out.join("8_npcs.bmp"), &renderer.rgba);
    }

    // --- 9. Только мир + спрайты (без HUD) — проверка видимости людей ---
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(
        &sprites,
        &[(Vec2::new(3.5, 2.7), NpcKind::Grandma), (Vec2::new(5.5, 2.7), NpcKind::Worker)],
        &cam,
        1.0,
    );
    renderer.present();
    write_bmp(&out.join("9_spriteonly.bmp"), &renderer.rgba);

    // --- 10. Камера вплотную к прилавку (0.15 тайла) — раньше при таком
    //     расстоянии он схлопывался до фиксированной высоты (баг «сжатия»).
    //     Спрайт бабушки вблизи — проверка масштаба людей.
    let close_cam = GameCam {
        x: 5.5,
        y: 4.15,
        angle: -std::f32::consts::FRAC_PI_2,
    };
    renderer.render(&map, &textures, &close_cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(3.5, 2.7), NpcKind::Grandma)], &close_cam, 1.0);
    renderer.present();
    write_bmp(&out.join("10_close.bmp"), &renderer.rgba);

    // --- 11. Камера вплотную к обычной стене (0.3 тайла) ---
    let wall_cam = GameCam {
        x: 1.3,
        y: 4.0,
        angle: 0.0,
    };
    renderer.render(&map, &textures, &wall_cam, 1.0);
    renderer.present();
    write_bmp(&out.join("11_wallclose.bmp"), &renderer.rgba);

    // --- 12. Полоска ускорения брожения (мини-игра на ПРОБЕЛЕ) ---
    let qte = BrewQte {
        active: true,
        pos: 0.48,
        dir: 1.0,
        zone0: 0.42,
        zone1: 0.54,
        barrel: Some(keys[1]),
        lock: 0.0,
        flash: 0.0,
    };
    let qte_state = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 12,
        norm: 90,
        day_earned: 12,
        time_left: 300.0,
        msg: "САХАР ДОБАВЛЕН. БРОДИТ ~70 СЕК. ПРОБЕЛ — УСКОРИТЬ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let qte_inv = Inventory {
        hands: [Some(HandItem::Bucket), None, None, None],
        selected: 0,
        stock_croutons: 2,
        stock_sugar: 1,
        stock_glass: 4,
        stock_jar: 2,
        stock_bottle: 1,
    };
    let qte_renders = vec![ui::CustomerRender {
        kind: NpcKind::Worker,
        pos: Vec2::new(5.5, 2.7),
        bubble: Some("Ну и как там, бродит?".into()),
    }];
    let qte_view = ui::CustomerView { list: &qte_renders };
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(
        &sprites,
        &[(Vec2::new(5.5, 2.7), NpcKind::Worker)],
        &cam,
        1.0,
    );
    ui::draw_frame(&mut renderer, &font, &qte_state, &qte_inv, &barrels, &qte, &qte_view, 6.0, &cam, &map);
    renderer.present();
    write_bmp(&out.join("12_qte.bmp"), &renderer.rgba);

    println!("selftest: кадры записаны в {}/", out.display());
}

fn write_bmp(path: &Path, rgba: &[u8]) {
    let (w, h) = (SCREEN_W, SCREEN_H);
    let row = (w * 3 + 3) & !3;
    let data_len = row * h;
    let file_len = 54 + data_len;
    let mut b = Vec::with_capacity(file_len);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(file_len as u32).to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&(data_len as u32).to_le_bytes());
    b.extend_from_slice(&2835u32.to_le_bytes());
    b.extend_from_slice(&2835u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..h).rev() {
        let start = b.len();
        for x in 0..w {
            let o = (y * w + x) * 4;
            b.push(rgba[o + 2]);
            b.push(rgba[o + 1]);
            b.push(rgba[o]);
        }
        while (b.len() - start) % 4 != 0 {
            b.push(0);
        }
    }
    fs::write(path, b).expect("записать bmp");
}
