//! Оффскрин-самотест: рисует кадры всех экранов в BMP/PNG-файлы и выходит.
//! Запуск: `cargo run -- --selftest`. Файлы в `selftest_out/`.
//!
//! Кадр = мир 640×400 (×2) + канвас UI 1280×800 поверх (альфа-композиция),
//! ровно как в игре: мир — нижний спрайт, UI — верхний.

use std::fs;
use std::path::Path;

use bevy::prelude::Vec2;

use crate::ambush::{AmbushStage, BanditAmbush, BanditControl};
use crate::brewing::{
    Barrel, BarrelPhase, Barrels, BrewQte, FirstOverbrew, FirstOverbrewStage,
};
use crate::canvas::{Canvas, CV_H, CV_W};
use crate::day_cycle::{
    DAY_INTRO_SECS, GameState, PAUSE_FROM_PLAYING, Phase, SPAWN_ANGLE, SPAWN_X, SPAWN_Y, norm_for,
};
use crate::debug::DebugConsole;
use crate::font::{Font, SZ_L, SZ_M, SZ_S};
use crate::inventory::{HandItem, Inventory};
use crate::interact::CoolQte;
use crate::map::Map;
use crate::modals::{Delivery, Modal, Modals, Sponge, SupplyKind};
use crate::raycaster::{GameCam, Renderer, SCREEN_H, SCREEN_W};
use crate::sprites::{NpcKind, SpriteSheet, Vessel};
use crate::textures::Textures;
use crate::ui;

/// Облачко/иконка заказа для UI (как в игре собирает render_frame).
fn cv(x: f32, y: f32, text: Option<&str>, order: Vessel, queued: bool) -> ui::CustView {
    ui::CustView {
        pos: Vec2::new(x, y),
        text: text.map(|s| s.to_string()),
        order,
        queued,
    }
}

pub fn run() {
    let out = Path::new("selftest_out");
    fs::create_dir_all(out).expect("создать selftest_out");
    let map = Map::shop();
    let textures = Textures::generate();
    let sprites = SpriteSheet::generate();
    let font = Font::new();
    eprintln!("{}", crate::BUILD_ID);
    eprintln!(
        "font metrics: {} — line(24)={} cap(24)={} w[КВАСНАЯ ЛАВКА](64)={} w[ШАГ](24)={}",
        font.px_label,
        font.line(SZ_M),
        font.cap_h(SZ_M),
        font.w("КВАСНАЯ ЛАВКА", 64),
        font.w("ШАГ 4/10. К БОЧКЕ: E ТРИЖДЫ (СУХАРИ-ВОДА-САХАР)", SZ_M),
    );
    let mut renderer = Renderer::new();
    let mut canvas = Canvas::new();
    let cam = GameCam {
        x: 7.0,
        y: 4.7,
        angle: -0.45, // взгляд на кассу (9,3) и очередь у правой стены
    };
    let no_qte = BrewQte::default();
    let no_cool = CoolQte::default();
    let no_modal = Modals::default();
    let no_props: &[crate::sprites::WorldProp] = &[];
    let no_cust: Vec<ui::CustView> = Vec::new();
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
        &mut canvas,
        &font,
        &state,
        &empty_inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        1.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "1_menu.bmp");

    // --- 1б. Меню с активным слотом «ПРОДОЛЖИТЬ» (есть прогресс) ---
    let state = GameState {
        phase: Phase::Menu,
        has_progress: true,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &empty_inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        1.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "1b_menu_continue.bmp");

    // --- 1в. Экран настроек (громкость) ---
    let state = GameState {
        phase: Phase::Menu,
        menu_view: crate::day_cycle::MenuView::Settings,
        menu_vol: 7,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &empty_inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        1.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "1c_menu_settings.bmp");

    // --- 1г. Окно рекомендации обучения (ВХОД до пройденного обучения) ---
    let state = GameState {
        phase: Phase::Menu,
        menu_view: crate::day_cycle::MenuView::Advice,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &empty_inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        1.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "1d_menu_advice.bmp");

    // --- 2. Окно снабжения (ТЗ №3): модалка поверх приглушённого мира ---
    let inv = Inventory {
        hands: [None; 4],
        selected: 0,
        stock_croutons: 2,
        stock_sugar: 2,
        stock_glass: 6,
        dirty_glasses: 1,
        dirty_jars: 1,
        dirty_bottles: 0,
        stock_jar: 2,
        stock_bottle: 1,
    };
    let state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 45,
        norm: norm_for(2),
        msg: "ЗАКАЗ ПРИНЯТ: СУХАРИ — ПРИВЕЗУТ ЧЕРЕЗ 3-5 СЕК".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let supply_modal = Modals {
        modal: Modal::Supply,
        pending: vec![
            Delivery {
                kind: SupplyKind::Croutons,
                t: 3.4,
            },
            Delivery {
                kind: SupplyKind::Sugar,
                t: 1.2,
            },
        ],
        ..Default::default()
    };
    let custs = vec![
        cv(
            5.5,
            2.7,
            Some("Дай, внучек, стаканчик квасика, холодненький..."),
            Vessel::Glass,
            true,
        ),
        cv(5.5, 1.7, None, Vessel::Jar, true),
    ];
    renderer.render(&map, &textures, &cam, 1.0);
    let sprite_list: Vec<(Vec2, NpcKind)> = vec![
        (Vec2::new(5.5, 2.7), NpcKind::Grandma),
        (Vec2::new(5.5, 1.7), NpcKind::Worker),
    ];
    renderer.draw_sprites(&sprites, &sprite_list, &cam, 1.0);
    renderer.present();
    let world_a = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &custs,
        2.0,
        &cam,
        &map,
        false,
        &supply_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_a), "2_supply.bmp");

    // --- 3. Стеллаж посуды (ТЗ №4): выбор посуды цифрами ---
    let pick_modal = Modals {
        modal: Modal::VesselPick,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &custs,
        2.4,
        &cam,
        &map,
        false,
        &pick_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_a), "3_vesselpick.bmp");

    // --- 4. Игровой день: мир + HUD + диалоги + иконка заказа (ТЗ №5) ---
    let mut barrels = Barrels::from_map(&map);
    let mut keys: Vec<(i32, i32)> = barrels.map.keys().copied().collect();
    keys.sort();
    // одна бочка (ТЗ redesign): в кадре — готовая с иконкой заказа (ТЗ №5)
    if let Some(b) = barrels.map.get_mut(&keys[0]) {
        b.phase = BarrelPhase::Ready(104.0);
        b.portions = 13;
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
        stock_glass: 5,
        dirty_glasses: 2,
        dirty_jars: 1,
        dirty_bottles: 1,
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
        cv(
            9.5,
            2.7,
            Some("Дай, внучек, стаканчик квасика, холодненький..."),
            Vessel::Glass,
            true,
        ),
        cv(
            9.5,
            1.7,
            Some("Стакан, но погуще, а то опять водица."),
            Vessel::Glass,
            true,
        ),
    ];
    renderer.render(&map, &textures, &cam, 1.0);
    let sprite_list: Vec<(Vec2, NpcKind)> = vec![
        (Vec2::new(9.5, 2.7), NpcKind::Grandma),
        (Vec2::new(9.5, 1.7), NpcKind::Worker),
    ];
    renderer.draw_sprites(&sprites, &sprite_list, &cam, 1.0);
    renderer.present();
    let world_b = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &custs,
        3.5,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_b), "4_playing.bmp");

    // --- 5. Итоги дня ---
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
        &mut canvas,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        4.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "5_dayend.bmp");

    // --- 6. Финал ---
    let state = GameState {
        phase: Phase::Final,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &Barrels::from_map(&map),
        &no_qte,
        &no_cool,
        &no_cust,
        5.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "6_final.bmp");

    // --- 7. Шрифт: кегли и образцы реальных строк ---
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
        "ШАГ 5/10. НАЖИМАЙТЕ ПРОБЕЛ В ЗОНУ БЕГУНКА",
        "ОКНО СНАБЖЕНИЯ: [1] СУХАРИ [2] САХАР",
        "СУХАРИ КОНЧИЛИСЬ — КУПИТЕ (1) НА СКЛАДЕ",
        "ПРИЛАВОК - ОТДАТЬ КВАС ПОКУПАТЕЛЮ",
        "Б2 ГОТОВА 13/20 12С",
        "Дай, внучек, стаканчик квасика, холодненький...",
    ] {
        println!("font.w({s:?}) = {} px", font.w(s, SZ_M));
    }
    let long = "ОКНО СНАБЖЕНИЯ: [1] СУХАРИ [2] САХАР";
    let fitted = font.fit(long, CV_W - 48, SZ_L);
    println!(
        "font.fit(длинная, 1232, 30) = {fitted} (тогда ширина {})",
        font.w(long, fitted)
    );
    snap(&canvas, None, "7_font.bmp");

    // --- 8. Обучение: шаг 5 (ТЗ №2) — ПРОБЕЛ в зону бегунка ---
    let state = GameState {
        phase: Phase::Tutorial,
        tut_step: 4,
        msg: "ТАК ДЕРЖАТЬ!".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let tut_cust: Vec<ui::CustView> = Vec::new();
    let tut_qte = BrewQte {
        active: true,
        pos: 0.47,
        dir: 1.0,
        zone0: 0.40,
        zone1: 0.53,
        barrel: Some(keys[0]),
        lock: 0.0,
        flash: 0.0,
    };
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.present();
    let world_tut = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &barrels,
        &tut_qte,
        &no_cool,
        &tut_cust,
        1.5,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_tut), "8_tutorial.bmp");

    // --- 8b. Обучение без мини-игры: плашка внизу, обзор свободен ---
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &tut_cust,
        1.5,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_tut), "8b_tutorial_bottom.bmp");

    // --- 8c. Обучение: шаг 9/10 — после гостя грязная посуда, ведём к мойке ---
    let sink_cam = GameCam {
        x: 2.7,
        y: 4.5,
        angle: std::f32::consts::PI, // взгляд на запад: мойка (1,4)
    };
    renderer.render(&map, &textures, &sink_cam, 1.0);
    renderer.present();
    let world_sink = renderer.rgba.clone();
    let sink_state = GameState {
        phase: Phase::Tutorial,
        tut_step: 8,
        msg: "ПОСЛЕ ГОСТЯ ОСТАЛАСЬ ГРЯЗНАЯ ПОСУДА — ОТНЕСИТЕ СТАКАН В МОЙКУ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let mut sink_inv = Inventory::default();
    sink_inv.dirty_glasses = 1;
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &sink_state,
        &sink_inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        2.0,
        &sink_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_sink), "8c_tutorial_sink.bmp");

    // --- 8d. Обучение: шаг 10/10 — все 4 ячейки заняты хламом, несём к урне ---
    let bin_cam = GameCam {
        x: 3.7,
        y: 6.4,
        angle: std::f32::consts::PI,
    };
    let bin_props = [crate::sprites::WorldProp {
        kind: crate::sprites::PropKind::TrashBin,
        pos: Vec2::new(2.5, 6.5),
    }];
    renderer.render(&map, &textures, &bin_cam, 1.0);
    renderer.draw_props(&sprites, &bin_props, &bin_cam, 1.0);
    renderer.present();
    let world_bin = renderer.rgba.clone();
    let bin_state = GameState {
        phase: Phase::Tutorial,
        tut_step: 9,
        msg: "РУКИ ПОЛНЫ ХЛАМА. ВЫБРОСЬТЕ ВСЁ В МУСОРКУ У СТЕНЫ (E)".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    let mut bin_inv = Inventory::default();
    bin_inv.hands = [
        Some(HandItem::Croutons(1)),
        Some(HandItem::Sugar(1)),
        Some(HandItem::Empty(Vessel::Jar, 1)),
        Some(HandItem::Empty(Vessel::Bottle, 1)),
    ];
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &bin_state,
        &bin_inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        2.4,
        &bin_cam,
        &map,
        false,
        &no_modal,
        &bin_props,
    );
    snap(&canvas, Some(&world_bin), "8d_tutorial_trash.bmp");

    // --- 9. Лист персонажей (проверка образов NPC) ---
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
            "БАБУШКА РАБОЧИЙ МАМА СТУДЕНТ МИЛИЦИОНЕР ХУЛИГАН",
            SZ_M,
            ui::col::WHITE,
            255,
        );
    }
    snap(&canvas, None, "9_npcs.bmp");

    // --- 10. Только мир + спрайты (без HUD) — очередь линией у прилавка ---
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.draw_sprites(
        &sprites,
        &[
            (Vec2::new(5.5, 2.7), NpcKind::Grandma),
            (Vec2::new(5.5, 1.7), NpcKind::Worker),
        ],
        &cam,
        1.0,
    );
    renderer.present();
    write_bmp_world(&out.join("10_spriteonly.bmp"), &renderer.rgba);

    // --- 11. Камера вплотную к прилавку (регрессия крышки/кассы) ---
    let close_cam = GameCam {
        x: 9.5,
        y: 4.5,
        angle: -std::f32::consts::FRAC_PI_2,
    };
    renderer.render(&map, &textures, &close_cam, 1.0);
    renderer.draw_sprites(&sprites, &[(Vec2::new(9.5, 2.7), NpcKind::Grandma)], &close_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("11_close.bmp"), &renderer.rgba);

    // --- 11b. Вид через прилавок на вход (2 клетки): кромка крышки не должна
    //         «ломаться» на стыке клеток двери (регрессия излома пространства) ---
    let door_cam = GameCam {
        x: 9.5,
        y: 4.6,
        angle: -std::f32::consts::FRAC_PI_2,
    };
    renderer.render(&map, &textures, &door_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("11b_door_seam.bmp"), &renderer.rgba);

    // --- 12. Камера вплотную к обычной стене ---
    let wall_cam = GameCam {
        x: 10.6,
        y: 3.5,
        angle: 0.0,
    };
    renderer.render(&map, &textures, &wall_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("12_wallclose.bmp"), &renderer.rgba);

    // --- 13. Ряд прилавков под острым углом: луч проходит несколько
    //         клеток подряд, все они должны быть видны (регрессия
    //         «невидимого куба» и исчезающих касс). ---
    let counter_cam = GameCam {
        x: 2.5,
        y: 3.5,
        angle: 0.245,
    };
    renderer.render(&map, &textures, &counter_cam, 1.0);
    renderer.present();
    write_bmp_world(&out.join("13_counter_row.bmp"), &renderer.rgba);

    // --- 13b. Внешний вид бочки по фазам: бродит / готово / переброд /
    //          испорчена. Одна камера, меняется только фаза. ---
    let bp = keys[0];
    let barrel_cam = GameCam {
        x: bp.0 as f32 + 0.5,
        y: bp.1 as f32 - 1.45,
        angle: std::f32::consts::FRAC_PI_2,
    };
    let mut look_barrels = Barrels::from_map(&map);
    for (name, phase) in [
        ("13b_barrel_ferment", BarrelPhase::Ferment(50.0)),
        ("13c_barrel_ready", BarrelPhase::Ready(112.0)),
        ("13d_barrel_overripe", BarrelPhase::Overripe(132.0)),
        ("13e_barrel_ruined", BarrelPhase::Ruined),
    ] {
        if let Some(b) = look_barrels.map.get_mut(&bp) {
            b.phase = phase;
        }
        renderer.render_with_barrels(
            &map,
            &textures,
            &barrel_cam,
            1.0,
            Some(&look_barrels),
        );
        renderer.present();
        write_bmp_world(&out.join(format!("{name}.bmp")), &renderer.rgba);
    }

    // --- 13f. Экран первого переброда: остановленное время, акцентные
    //          цифры и стрелка к колонке (ведра ещё нет). ---
    let event = FirstOverbrew {
        stage: FirstOverbrewStage::NeedBucket,
        barrel: bp,
    };
    let event_state = GameState {
        phase: Phase::Playing,
        day: 1,
        time_left: 180.0,
        ..Default::default()
    };
    let event_inv = Inventory::default();
    let mut event_barrels = Barrels::from_map(&map);
    if let Some(b) = event_barrels.map.get_mut(&bp) {
        b.phase = BarrelPhase::Overripe(126.0);
    }
    renderer.render_with_barrels(&map, &textures, &barrel_cam, 1.0, Some(&event_barrels));
    renderer.present();
    let world_event = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame_with_overbrew(
        &mut canvas,
        &font,
        &event_state,
        &event_inv,
        &event_barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        6.0,
        &barrel_cam,
        &map,
        false,
        &no_modal,
        &no_props,
        Some(&event),
    );
    snap(&canvas, Some(&world_event), "13f_overbrew_event.bmp");

    // --- 13g. Ведро уже в руках: цель — бочка, сценарий ждёт E. ---
    let pour_event = FirstOverbrew {
        stage: FirstOverbrewStage::PourWater,
        barrel: bp,
    };
    let pour_inv = Inventory {
        hands: [Some(HandItem::Bucket), None, None, None],
        selected: 0,
        ..Default::default()
    };
    if let Some(b) = event_barrels.map.get_mut(&bp) {
        b.phase = BarrelPhase::Overripe(126.0);
    }
    renderer.render_with_barrels(&map, &textures, &barrel_cam, 1.0, Some(&event_barrels));
    renderer.present();
    let world_pour = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame_with_overbrew(
        &mut canvas,
        &font,
        &event_state,
        &pour_inv,
        &event_barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        6.0,
        &barrel_cam,
        &map,
        false,
        &no_modal,
        &no_props,
        Some(&pour_event),
    );
    snap(&canvas, Some(&world_pour), "13g_overbrew_pour.bmp");

    // --- 13h. Провал: бочка испорчена, время ждёт мытья. ---
    let failed_event = FirstOverbrew {
        stage: FirstOverbrewStage::FailedWash,
        barrel: bp,
    };
    if let Some(b) = event_barrels.map.get_mut(&bp) {
        b.phase = BarrelPhase::Ruined;
    }
    renderer.render_with_barrels(&map, &textures, &barrel_cam, 1.0, Some(&event_barrels));
    renderer.present();
    let world_failed = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame_with_overbrew(
        &mut canvas,
        &font,
        &event_state,
        &event_inv,
        &event_barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        6.0,
        &barrel_cam,
        &map,
        false,
        &no_modal,
        &no_props,
        Some(&failed_event),
    );
    snap(&canvas, Some(&world_failed), "13h_overbrew_failed.bmp");

    // --- 14. Полоска ускорения брожения (мини-игра на ПРОБЕЛЕ) ---
    let qte = BrewQte {
        active: true,
        pos: 0.48,
        dir: 1.0,
        zone0: 0.42,
        zone1: 0.54,
        barrel: Some(keys[0]),
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
        dirty_glasses: 0,
        dirty_jars: 0,
        dirty_bottles: 0,
        stock_jar: 2,
        stock_bottle: 1,
    };
    let qte_cust = vec![cv(
        5.5,
        2.7,
        Some("Ну и как там, бродит?"),
        Vessel::Glass,
        true,
    )];
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
        &no_cool,
        &qte_cust,
        6.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_c), "13_qte.bmp");

    // --- 14. Мойка посуды (ТЗ №4): губка протирает сетку грязи 8×8 ---
    let mut sponge = Sponge::begin(Vessel::Glass);
    sponge.cx = 3.2;
    sponge.cy = 3.4;
    let _ = sponge.erase();
    sponge.cx = 5.0;
    sponge.cy = 4.6;
    let _ = sponge.erase();
    let sponge_modal = Modals {
        modal: Modal::Sponge,
        sponge,
        ..Default::default()
    };
    let wash_state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 30,
        norm: norm_for(2),
        day_earned: 40,
        time_left: 200.0,
        msg: "МОЕМ СТАКАН: WASD ВЕДУТ ГУБКУ ПО СЕТКЕ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &wash_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        6.4,
        &cam,
        &map,
        false,
        &sponge_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_c), "14_sponge.bmp");

    // --- 15. Охлаждение перебродившей бочки (ТЗ №9): синяя зона ---
    let mut cool_barrels = Barrels::from_map(&map);
    if let Some(b) = cool_barrels.map.get_mut(&keys[0]) {
        b.phase = BarrelPhase::Overripe(130.0);
        b.portions = 20;
    }
    let cool = CoolQte {
        active: true,
        pos: 0.49,
        dir: 1.0,
        zone0: 0.38,
        zone1: 0.52,
        barrel: Some(keys[0]),
        lock: 0.0,
        flash: 0.0,
    };
    let cool_state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 30,
        norm: norm_for(2),
        day_earned: 40,
        time_left: 150.0,
        msg: "ПЕРЕБРОД 130% — ПРОБЕЛ В СИНЮЮ ЗОНУ (ВЕДРО ЛЬЁТСЯ)".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &cool_state,
        &qte_inv,
        &cool_barrels,
        &no_qte,
        &cool,
        &no_cust,
        6.8,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_c), "15_coolqte.bmp");

    // --- 16. Пауза (ESC): меню поверх приглушённого мира ---
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
        &no_cool,
        &custs,
        3.5,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_b), "16_pause.bmp");

    // --- 17. Вся посуда грязная: предупреждение сверху + красные счётчики ---
    let no_inv = Inventory {
        hands: [None; 4],
        selected: 0,
        stock_croutons: 0,
        stock_sugar: 0,
        stock_glass: 0,
        dirty_glasses: 3,
        dirty_jars: 2,
        dirty_bottles: 1,
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
        msg: "ПОСУДА ВСЯ ГРЯЗНАЯ! ИДИТЕ НА МОЙКУ (E)".into(),
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
        &no_cool,
        &no_cust,
        2.2,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_c), "17_noposuda.bmp");

    // --- 18. Выбор грязной посуды на мойке (ТЗ №4): три карточки ---
    let pick_modal = Modals {
        modal: Modal::Sponge,
        sponge: Sponge::default(), // стадия выбора
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &warn_state,
        &no_inv,
        &empty_barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        2.6,
        &cam,
        &map,
        false,
        &pick_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_c), "18_spongepick.bmp");

    // --- 19. Мусорка (ТЗ №5): урна — билборд-спрайт в уголке у южной стены,
    //         игрок подошёл: виден спрайт и подсказка «E МУСОРКА...» ---
    let trash_cam = GameCam {
        x: 3.7,
        y: 6.4,
        angle: std::f32::consts::PI, // взгляд на запад: урна (2.5,6.5) в 1.2 м — уже в радиусе E
    };
    let trash_props = [crate::sprites::WorldProp {
        kind: crate::sprites::PropKind::TrashBin,
        pos: Vec2::new(2.5, 6.5),
    }];
    renderer.render(&map, &textures, &trash_cam, 1.0);
    renderer.draw_props(&sprites, &trash_props, &trash_cam, 1.0);
    renderer.present();
    let world_trash = renderer.rgba.clone();
    let trash_state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 45,
        norm: norm_for(2),
        day_earned: 40,
        time_left: 300.0,
        msg: "МУСОРКА РЯДОМ — ВЫБРОСИТЬ НЕНУЖНОЕ (E)".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &trash_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        5.0,
        &trash_cam,
        &map,
        false,
        &no_modal,
        &trash_props,
    );
    snap(&canvas, Some(&world_trash), "19_trash.bmp");

    // --- 20. «Квас кончился» (ТЗ №1): бочка сразу помечена на мойку ---
    let mut ran_barrels = Barrels::from_map(&map);
    if let Some(b) = ran_barrels.map.get_mut(&keys[0]) {
        b.phase = BarrelPhase::Ruined;
        b.portions = 0;
        b.ran_out = true;
    }
    renderer.render(&map, &textures, &cam, 1.0);
    renderer.present();
    let world_ran = renderer.rgba.clone();
    let ran_state = GameState {
        phase: Phase::Playing,
        day: 3,
        money: 21,
        norm: norm_for(3),
        day_earned: 40,
        time_left: 240.0,
        msg: "КВАС КОНЧИЛСЯ! ПОМОЙТЕ БОЧКУ (ДЕРЖИТЕ E)".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &ran_state,
        &inv,
        &ran_barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.1,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_ran), "20_ranout.bmp");

    // --- 21. Подсказка у прилавка (ТЗ №6): ничего не занято — панель есть ---
    let hint_cam = GameCam {
        x: 9.5,
        y: 4.6,
        angle: -std::f32::consts::FRAC_PI_2, // взгляд на север: прилавок (9,3)
    };
    renderer.render(&map, &textures, &hint_cam, 1.0);
    renderer.present();
    let world_hint = renderer.rgba.clone();
    let hint_state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 45,
        norm: norm_for(2),
        day_earned: 40,
        time_left: 300.0,
        msg: "КЛИЕНТ ЖДЁТ ЗА ПРИЛАВКОМ".into(),
        msg_t: 3.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &hint_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        4.2,
        &hint_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_hint), "21_hint.bmp");

    // --- 22. Тревога (ТЗ №5): бандиты на подходе — красная пульсация экрана ---
    let alarm_state = GameState {
        phase: Phase::Playing,
        day: 2,
        money: 45,
        norm: norm_for(2),
        day_earned: 40,
        time_left: 300.0,
        msg: "БАНДИТЫ НА ПОДХОДЕ! БЕГИТЕ К КАССЕ (E) - ЗАПЛАТИТЕ ДАНЬ".into(),
        msg_t: 3.0,
        alarm_t: 2.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &alarm_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        4.4,
        &hint_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_hint), "22_alarm.bmp");

    // --- 23. Заставка начала дня (ТЗ: «ДЕНЬ 1»): сначала полная тьма ---
    let spawn_cam = GameCam {
        x: SPAWN_X,
        y: SPAWN_Y,
        angle: SPAWN_ANGLE,
    };
    renderer.render(&map, &textures, &spawn_cam, 1.0);
    renderer.present();
    let world_spawn = renderer.rgba.clone();
    let intro_state = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 30,
        norm: norm_for(1),
        time_left: 240.0,
        day_intro: DAY_INTRO_SECS,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &intro_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        0.4,
        &spawn_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_spawn), "23_day_intro.bmp");

    // --- 23b. Та же заставка на середине: тьма и надпись гаснут вместе ---
    let intro_half = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 30,
        norm: norm_for(1),
        time_left: 240.0,
        day_intro: DAY_INTRO_SECS - 2.0, // ~половина проявления
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &intro_half,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        2.0,
        &spawn_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_spawn), "23b_day_intro_half.bmp");

    // --- 23c. Вид на стартовой точке после заставки (магазин открыт) ---
    let after_intro = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 30,
        norm: norm_for(1),
        time_left: 240.0,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &after_intro,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.4,
        &spawn_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_spawn), "23c_spawn_view.bmp");

    // --- 23d. Склад: сухари на северной стене, сахар на южной, вода справа ---
    let storage_cam = GameCam {
        x: 14.3,
        y: 7.7,
        angle: -0.65,
    };
    renderer.render(&map, &textures, &storage_cam, 1.0);
    renderer.present();
    let world_storage = renderer.rgba.clone();
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &after_intro,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.4,
        &storage_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, Some(&world_storage), "23d_storage.bmp");

    // --- 23e. Предыстория перед первым днём ---
    let story_state = GameState {
        phase: Phase::Story,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &story_state,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        1.0,
        &cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    snap(&canvas, None, "23e_story.bmp");

    // --- 23f. Отладочная консоль поверх игры ---
    let dbg = DebugConsole {
        open: true,
        input: "day 2".into(),
        log: vec![
            "КОНСОЛЬ ОТЛАДКИ".into(),
            "day 1..3 | stop | play | band | unband".into(),
            "ДЕНЬ 1 ЗАГРУЖЕН".into(),
        ],
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &after_intro,
        &inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.4,
        &storage_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    let dbg_control = BanditControl::default();
    ui::draw_debug_console(&mut canvas, &font, &dbg, &dbg_control, 1, 3.4);
    snap(&canvas, Some(&world_storage), "23f_debug.bmp");

    // --- 23g. Бандит в коридоре: выбор ответа ---
    let bandit_inv = Inventory {
        hands: [Some(HandItem::Bucket), None, None, None],
        selected: 0,
        ..Default::default()
    };
    let bandit_choice = BanditAmbush {
        stage: AmbushStage::Choice,
        phrase: 0,
        choice: 1,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &after_intro,
        &bandit_inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.4,
        &storage_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    ui::draw_ambush(&mut canvas, &font, &after_intro, &bandit_inv, &bandit_choice);
    snap(&canvas, Some(&world_storage), "23g_ambush_choice.bmp");

    // --- 23h. Ложь не сработала: подбитый глаз ---
    let hurt = GameState {
        phase: Phase::Playing,
        day: 1,
        money: 30,
        norm: norm_for(1),
        time_left: 240.0,
        black_eye: 45.0,
        ..Default::default()
    };
    let bandit_result = BanditAmbush {
        stage: AmbushStage::Result,
        phrase: 2,
        result: format!(
            "ВЫ: «{}»\n\nБАНДИТ: Заливаешь, уборщик?! Получай!",
            crate::ambush::LIE_LINE
        ),
        black_eye_hit: true,
        ..Default::default()
    };
    canvas.clear();
    ui::draw_frame(
        &mut canvas,
        &font,
        &hurt,
        &bandit_inv,
        &barrels,
        &no_qte,
        &no_cool,
        &no_cust,
        3.4,
        &storage_cam,
        &map,
        false,
        &no_modal,
        &no_props,
    );
    ui::draw_black_eye(&mut canvas, 3.4, hurt.black_eye);
    ui::draw_ambush(&mut canvas, &font, &hurt, &bandit_inv, &bandit_result);
    snap(&canvas, Some(&world_storage), "23h_ambush_black_eye.bmp");

    // --- 24. Логика начала дня: телепорт на спавн + заставка «ДЕНЬ N» ---
    assert_eq!(crate::tutorial::TUT_STEPS.len(), 10, "в обучении 10 шагов");
    assert_eq!(crate::tutorial::TUT_STEPS[8].target, (8.5, 5.5), "шаг 9 — мойка");
    assert_eq!(crate::tutorial::TUT_STEPS[9].target, (5.5, 8.3), "шаг 10 — мусорка");

    // Регрессия «ловушки QTE»: после 100% ускорять бочку нельзя.
    let mut probe = Barrel::new();
    probe.phase = BarrelPhase::Ferment(50.0);
    assert!(probe.boostable(), "бродящую бочку можно ускорять");
    probe.phase = BarrelPhase::Ready(110.0);
    assert!(
        !probe.boostable(),
        "готовую бочку нельзя ускорять: иначе 118% + QTE = переброд"
    );
    probe.phase = BarrelPhase::Overripe(130.0);
    assert!(!probe.boostable(), "переброд ускорять нельзя");
    assert!(
        crate::brewing::FIRST_OVERBREW_RATE < crate::brewing::DANGER_RATE,
        "первый переброд медленнее обычного"
    );
    let mut first = FirstOverbrew::default();
    assert!(!first.freeze(), "до события время идёт нормально");
    first.stage = FirstOverbrewStage::NeedBucket;
    assert!(first.freeze(), "без ведра время стоит");
    first.stage = FirstOverbrewStage::PourWater;
    assert!(first.freeze(), "с ведром время ещё стоит до полива");
    first.stage = FirstOverbrewStage::FailedWash;
    assert!(first.freeze(), "после провала время стоит до мытья");
    first.stage = FirstOverbrewStage::Done;
    assert!(!first.freeze(), "после спасения/мытья время идёт");
    let mut st = GameState::default();
    let mut day_barrels = Barrels::from_map(&map);
    let mut day_dirty = crate::day_cycle::Dirty::default();
    let mut day_cam = GameCam {
        x: 0.0,
        y: 0.0,
        angle: 0.0,
    };
    crate::day_cycle::start_day(&mut st, &mut day_barrels, &mut day_dirty, &mut day_cam);
    assert_eq!(st.phase, Phase::Playing);
    assert_eq!(st.day_intro, DAY_INTRO_SECS);
    assert!(
        (day_cam.x - SPAWN_X).abs() < 1e-4
            && (day_cam.y - SPAWN_Y).abs() < 1e-4
            && (day_cam.angle - SPAWN_ANGLE).abs() < 1e-4,
        "start_day телепортирует на стартовую точку"
    );
    println!(
        "logic: start_day OK (phase={:?}, intro={}s, cam=({:.1},{:.1},{:.2}))",
        st.phase, st.day_intro, day_cam.x, day_cam.y, day_cam.angle
    );

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
    // тот же кадр в PNG — просмотр BMP невозможен в чате/просмотрщике
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let o = (y * w + x) * 4;
            rgb.push(rgba[o]);
            rgb.push(rgba[o + 1]);
            rgb.push(rgba[o + 2]);
        }
    }
    let png = path.with_extension("png");
    crate::dump::write_png(&png, w, h, &rgb).expect("записать png");
}
