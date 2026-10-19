//! Оффскрин-самотест: рисует кадры всех экранов в BMP-файлы и выходит.
//! Запуск: `cargo run -- --selftest`. Файлы в `selftest_out/`.
//!
//! Кадр = мир 640×400 (×2) + канвас UI 1280×800 поверх (альфа-композиция),
//! ровно как в игре: мир — нижний спрайт, UI — верхний.

use std::fs;
use std::path::Path;

use bevy::prelude::Vec2;

use crate::brewing::{BarrelPhase, Barrels, BrewQte};
use crate::canvas::{Canvas, CV_H, CV_W};
use crate::day_cycle::{GameState, Phase, PAUSE_FROM_PLAYING, norm_for};
use crate::font::{Font, SZ_L, SZ_M, SZ_S};
use crate::inventory::{HandItem, Inventory};
use crate::map::Map;
use crate::raycaster::{GameCam, Renderer, SCREEN_H, SCREEN_W};
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
    eprintln!(
        "font metrics: {} — line(24)={} cap(24)={} w[КВАСНАЯ ЛАВКА](64)={} w[ШАГ](24)={}",
        font.px_label,
        font.line(SZ_M),
        font.cap_h(SZ_M),
        font.w("КВАСНАЯ ЛАВКА", 64),
        font.w("ШАГ 4/7. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)", SZ_M),
    );
    let mut renderer = Renderer::new();
    let mut canvas = Canvas::new();
    let cam = GameCam {
        x: 5.5,
        y: 4.8,
        angle: -std::f32::consts::FRAC_PI_2 - 0.4,
    };
    let no_qte = BrewQte::default();
    let no_cust: Vec<(Vec2, Option<String>)> = Vec::new();
    let empty_inv = Inventory::default();

    //snap: композиция кадра и запись
    let snap = |canvas: &Canvas, world: Option<&[u8]>, name: &str| {
        let final_rgba = match world {
            Some(w) => canvas.over_world(w),
            None => canvas.px.clone(),
        };
        write_bmp(&out.join(name), &final_rgba);
    };

    // --- 1. Главное меню ---
    let state = GameState {
        phase: Phase::Menu,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas, &font, &state, &empty_inv, &Barrels::from_map(&map),
        &no_qte, &no_cust, 1.0, &cam, &map, false,
    );
    snap(&canvas, None, "1_menu.bmp");

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
        money: 45,
        norm: norm_for(1),
        msg: "КУПЛЕНО: ПОСУДА (4+4+4) -15 К.".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas, &font, &state, &inv, &Barrels::from_map(&map),
        &no_qte, &no_cust, 2.0, &cam, &map, false,
    );
    snap(&canvas, None, "2_shopping.bmp");

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
            if let Some(b) = barrels.map.get_mut(b1) {
                b.phase = BarrelPhase::Ready(104.0);
                b.portions = 13;
            }
        }
    }
    let inv = Inventory {
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
        norm: norm_for(2),
        day_earned: 60,
        time_left: 137.0,
        msg: "СУХАРИ КОНЧИЛИСЬ — КУПИТЕ (1) НА СКЛАДЕ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let custs = vec![
        (
            Vec2::new(3.5, 2.7),
            Some("Дай, внучек, стаканчик квасика, холодненький...".into()),
        ),
        (
            Vec2::new(5.5, 2.7),
            Some("Стакан, но погуще, а то опять водица.".into()),
        ),
    ];
    renderer.render(&map, &textures, &cam, 1.0);
    let sprite_list: Vec<(Vec2, NpcKind)> = vec![
        (Vec2::new(3.5, 2.7), NpcKind::Grandma),
        (Vec2::new(5.5, 2.7), NpcKind::Worker),
    ];
    renderer.draw_sprites(&sprites, &sprite_list, &cam, 1.0);
    renderer.present();
    let world_a = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(&mut canvas, &font, &state, &inv, &barrels, &no_qte, &custs, 3.5, &cam, &map, false);
    snap(&canvas, Some(&world_a), "3_playing.bmp");

    // --- 4. Итоги дня ---
    let state = GameState {
        phase: Phase::DayEnd,
        day: 2,
        money: 147,
        norm: norm_for(2),
        day_earned: 117,
        fine: 30,
        fine_reason: "ТЕЧЬ В БОЧКЕ №2 (ПЕРЕБРОД)".into(),
        day_passed: true,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas, &font, &state, &inv, &Barrels::from_map(&map),
        &no_qte, &no_cust, 4.0, &cam, &map, false,
    );
    snap(&canvas, None, "4_dayend.bmp");

    // --- 5. Финал ---
    let state = GameState {
        phase: Phase::Final,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas, &font, &state, &inv, &Barrels::from_map(&map),
        &no_qte, &no_cust, 5.0, &cam, &map, false,
    );
    snap(&canvas, None, "5_final.bmp");

    // --- 6. Шрифт: кегли и образцы реальных строк ---
    canvas.clear();
    canvas.rect(0, 0, CV_W, CV_H, [36, 30, 22], 255);
    let rows: [(&str, usize); 8] = [
        ("АБВГДЕЁЖЗИЙКЛМНОПРСТУ 0123456789", 30),
        ("ФХЦЧШЩЪЫЬЭЮЯ abcdefghijklmnopqrstuvwxyz", 24),
        ("правильный многонациональный текст ЁЙЩЭ №«»", 24),
        ("КАВС ДО КОНЦА - МОНОПОЛИЯ СОВЕТСКАЯ", 30),
        ("мелкая надпись на ценнике — базовый кегль 20", SZ_S),
        ("ШРИФТ ЧИТАЕМЫЙ ПИКСЕЛЬНЫЙ БЕЗ ЛАПШИ", 38),
        ("КВАСНАЯ ЛАВКА", 48),
        ("КВАСНАЯ ЛАВКА", 64),
    ];
    let mut ry = 24i32;
    for (r, s) in rows {
        font.draw_sh(&mut canvas, 24, ry, r, s, ui::col::WHITE, 255);
        ry += font.line(s) + 14;
    }
    for s in [
        "ШАГ 4/7. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)",
        "НАЧАЛО: ЗАКУПИТЕ СУХАРИ (1), САХАР (2) И ПОСУДУ (3) - ИЛИ СРАЗУ ЖМИТЕ ENTER",
        "СУХАРИ КОНЧИЛИСЬ — КУПИТЕ (1) НА СКЛАДЕ",
        "ПРИЛАВОК - ОТДАТЬ КВАС ПОКУПАТЕЛЮ",
        "Б2 ГОТОВА 13/20 12С",
        "Дай, внучек, стаканчик квасика, холодненький...",
    ] {
        println!("font.w({s:?}) = {} px", font.w(s, SZ_M));
    }
    let long = "НАЧАЛО: ЗАКУПИТЕ СУХАРИ (1), САХАР (2) И ПОСУДУ (3) - ИЛИ СРАЗУ ЖМИТЕ ENTER";
    let fitted = font.fit(long, CV_W - 48, SZ_L);
    println!(
        "font.fit(длинная, 1232, 30) = {fitted} (тогда ширина {})",
        font.w(long, fitted)
    );
    snap(&canvas, None, "6_font.bmp");

    // --- 7. Обучение: панель инструкции + мигающий маркер ---
    let state = GameState {
        phase: Phase::Tutorial,
        tut_step: 6,
        msg: "ТАК ДЕРЖАТЬ!".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let tut_cust = vec![(Vec2::new(3.5, 2.7), None)];
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(3.5, 2.7), NpcKind::Grandma)], &cam, 1.0);
    renderer.present();
    let world_b = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas, &font, &state, &inv, &barrels, &no_qte, &tut_cust, 1.5, &cam, &map, false,
    );
    snap(&canvas, Some(&world_b), "7_tutorial.bmp");

    // --- 8. Лист персонажей (проверка образов NPC) ---
    canvas.clear();
    canvas.rect(0, 0, CV_W, CV_H, [36, 30, 22], 255);
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
        for (i, k) in kinds.iter().enumerate() {
            let spr = sprites.npc_sprite(*k);
            let ox = 60 + i as i32 * 172;
            for y in 0..64 {
                for x in 0..32 {
                    let color = spr[y * 32 + x];
                    if color == crate::sprites::TRANSPARENT {
                        continue;
                    }
                    let rgb = crate::palette::PALETTE[color as usize];
                    canvas.rect(ox + x as i32 * 4, 60 + y as i32 * 4, 4, 4, rgb, 255);
                }
            }
            font.draw_sh(&mut canvas, ox - 10, 340, k.name(), SZ_S, ui::col::LAMP, 255);
        }
        font.draw(
            &mut canvas,
            60,
            400,
            "БАБУШКА РАБОЧИЙ МАМА СТУДЕНТ МИЛИЦИОНЕР ХУЛИГАН САНЭПИД",
            SZ_M,
            ui::col::WHITE,
            255,
        );
    }
    snap(&canvas, None, "8_npcs.bmp");

    // --- 9. Только мир + спрайты (без HUD) — проверка видимости людей ---
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(
        &sprites,
        &[(Vec2::new(3.5, 2.7), NpcKind::Grandma), (Vec2::new(5.5, 2.7), NpcKind::Worker)],
        &cam,
        1.0,
    );
    renderer.present();
    write_bmp_world(&out.join("9_spriteonly.bmp"), &renderer.rgba);

    // --- 10. Камера вплотную к прилавку — проверка масштаба людей ---
    let close_cam = GameCam {
        x: 5.5,
        y: 4.15,
        angle: -std::f32::consts::FRAC_PI_2,
    };
    renderer.render(&map, &textures, &close_cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(3.5, 2.7), NpcKind::Grandma)], &close_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("10_close.bmp"), &renderer.rgba);

    // --- 11. Камера вплотную к обычной стене ---
    let wall_cam = GameCam {
        x: 1.3,
        y: 4.0,
        angle: 0.0,
    };
    renderer.render(&map, &textures, &wall_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("11_wallclose.bmp"), &renderer.rgba);

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
        norm: norm_for(1),
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
    let qte_cust = vec![(Vec2::new(5.5, 2.7), Some("Ну и как там, бродит?".into()))];
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(5.5, 2.7), NpcKind::Worker)], &cam, 1.0);
    renderer.present();
    let world_c = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &qte_state,
        &qte_inv,
        &barrels,
        &qte,
        &qte_cust,
        6.0,
        &cam,
        &map, false,
    );
    snap(&canvas, Some(&world_c), "12_qte.bmp");

    // --- 13. Пауза (ESC): меню поверх приглушенного мира ---
    let pause_state = GameState {
        phase: Phase::Paused,
        day: 2,
        money: 47,
        norm: norm_for(2),
        day_earned: 60,
        time_left: 137.0,
        pause_prev: PAUSE_FROM_PLAYING,
        pause_sel: 0,
        tut_done: true,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &pause_state,
        &inv,
        &barrels,
        &no_qte,
        &custs,
        3.5,
        &cam,
        &map, false,
    );
    snap(&canvas, Some(&world_a), "13_pause.bmp");

    // --- 14. Посуда кончилась: предупреждение сверху + красные нули на складе ---
    let no_inv = Inventory {
        hands: [None; 4],
        selected: 0,
        stock_croutons: 0,
        stock_sugar: 0,
        stock_glass: 0,
        stock_jar: 0,
        stock_bottle: 0,
    };
    let warn_state = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 5,
        norm: norm_for(1),
        day_earned: 0,
        time_left: 60.0,
        msg: "ПОСУДА КОНЧИЛАСЬ — КУПИТЕ (3) НА СКЛАДЕ ЗАВТРА".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let empty_barrels = Barrels::from_map(&map);
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &warn_state,
        &no_inv,
        &empty_barrels,
        &no_qte,
        &no_cust,
        2.2,
        &cam,
        &map, false,
    );
    snap(&canvas, Some(&world_c), "14_noposuda.bmp");

    println!("selftest: кадры записаны в {}/", out.display());
}

/// Кадр мира 640×400 (нижний слой, без UI).
fn write_bmp_world(path: &Path, rgba: &[u8]) {
    write_bmp_sized(path, rgba, SCREEN_W, SCREEN_H);
}

fn write_bmp(path: &Path, rgba: &[u8]) {
    write_bmp_sized(path, rgba, CV_W as usize, CV_H as usize);
}

fn write_bmp_sized(path: &Path, rgba: &[u8], w: usize, h: usize) {
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
