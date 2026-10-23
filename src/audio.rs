//! Звук: процедурная генерация .wav (22 кГц, моно) + воспроизведение.
//! Музыка главного меню — FLAC, встроенный в exe (см. `MENU_MUSIC`).
//! Позже можно заменить файлы своими сэмплами — имена те же.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::{AudioSink, AudioSinkPlayback, GlobalVolume, Volume};
use bevy::prelude::*;

use crate::day_cycle::{GameState, Phase};

const SR: u32 = 22050;

pub struct Sfx;

/// Музыка главного меню (тема «Квасной лавки», FLAC).
/// Встроена прямо в exe: игра работает из любого каталога запуска
/// и не зависит от папки assets на диске.
pub const MENU_MUSIC: &[u8] = include_bytes!("../assets/music/menu_theme.flac");

/// Музыка основного игрового дня (тема «Квасной лавки», FLAC, 20 с, петля).
/// Тоже встроена в exe.
pub const GAME_MUSIC: &[u8] = include_bytes!("../assets/music/game_theme.flac");

/// Музыка экрана итогов дня (отчёт смены, FLAC, ~27 с, петля).
pub const DAY_END_MUSIC: &[u8] = include_bytes!("../assets/music/day_end.flac");

pub const SOUNDS: &[&str] = &[
    "pour",
    "croutons",
    "water",
    "hiss",
    "ding",
    "hiss2",
    "fooff",
    "coin",
    "step1",
    "step2",
    "door",
    "whistle",
    "pop",
    "bubble_loop",
    "street_loop",
    "alarm",
    "horn",
    "sting",
    "tension_loop",
    "punch",
];

fn write_wav(path: &PathBuf, samples: &[f32]) {
    if path.exists() {
        return;
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).expect("wav create"));
    let data_len = (samples.len() * 2) as u32;
    let sr = SR;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32000.0) as i16;
        f.write_all(&v.to_le_bytes()).unwrap();
    }
}

/// Детерминированный шум (xorshift).
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn env(t: f32, dur: f32, attack: f32, release: f32) -> f32 {
    let a = (t / attack).min(1.0);
    let r = ((dur - t) / release).min(1.0);
    a.max(0.0) * r.max(0.0)
}

/// Генерирует все звуки в <assets_root>/sounds (если файлов нет).
/// Пишем именно туда, откуда читает AssetServer: иначе при запуске
/// `dist\kvass-shop.exe` из другой папки новые звуки не находились.
pub fn generate_files() {
    let dir = crate::assets_root().join("sounds");
    std::fs::create_dir_all(&dir).unwrap();

    // литьё кваса: шипение + бульканье
    {
        let dur = 1.3;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(12345);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let x = noise.next();
            lp = lp * 0.7 + x * 0.3;
            let fizz = x * 0.5 + lp * 0.5;
            let gurgle = (t * 90.0 + (t * 7.0).sin() * 4.0).sin() * 0.25;
            let s = (fizz * 0.5 + gurgle) * env(t, dur, 0.05, 0.3) * 0.8;
            out.push(s);
        }
        write_wav(&dir.join("pour.wav"), &out);
    }
    // засыпание сухарей: сухой шорох
    {
        let dur = 0.45;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(777);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let x = noise.next();
            let crack = if noise.next() > 0.92 { noise.next() } else { 0.0 };
            let s = (x * 0.25 + crack * 0.75) * env(t, dur, 0.01, 0.25) * 0.7;
            out.push(s);
        }
        write_wav(&dir.join("croutons.wav"), &out);
    }
    // вода из ведра: ниже и с бульканьем
    {
        let dur = 0.7;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(4242);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            lp = lp * 0.85 + noise.next() * 0.15;
            let gurgle = ((t * 55.0).sin() * (t * 13.0).sin()).max(0.0) * 0.5;
            let s = (lp * 0.6 + gurgle * 0.4) * env(t, dur, 0.03, 0.3) * 0.8;
            out.push(s);
        }
        write_wav(&dir.join("water.wav"), &out);
    }
    // «пшшш» — брожение пошло на взлёт
    {
        let dur = 0.5;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(999);
        let mut hp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let x = noise.next();
            hp = x - hp * 0.2;
            let s = hp * env(t, dur, 0.02, 0.2) * 0.6;
            out.push(s);
        }
        write_wav(&dir.join("hiss.wav"), &out);
    }
    // «плюх/пип» — квас готов
    {
        let dur = 0.3;
        let n = (SR as f32 * dur) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let f = if t < 0.1 { 880.0 } else { 1320.0 };
            let sq = ((t * f * 6.28318).sin().signum()) * 0.5;
            let s = sq * env(t, dur, 0.005, 0.12) * 0.7;
            out.push(s);
        }
        write_wav(&dir.join("ding.wav"), &out);
    }
    // нарастающее шипение — окно переброда
    {
        let dur = 0.9;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(31337);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            lp = lp * 0.6 + noise.next() * 0.4;
            let gain = (t / dur).powf(1.5);
            let s = lp * gain * env(t, dur, 0.05, 0.05) * 0.7;
            out.push(s);
        }
        write_wav(&dir.join("hiss2.wav"), &out);
    }
    // «фьююю» — переброд, испорчено
    {
        let dur = 1.0;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(5150);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let k = 0.9 - (t / dur) * 0.7;
            lp = lp * k + noise.next() * (1.0 - k);
            let low = lp * 0.7 + (t * (300.0 - t * 200.0)).sin() * 0.3;
            let s = low * env(t, dur, 0.03, 0.6) * 0.8;
            out.push(s);
        }
        write_wav(&dir.join("fooff.wav"), &out);
    }
    // звон монетки
    {
        let dur = 0.25;
        let n = (SR as f32 * dur) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let f = if t < 0.08 { 2093.0 } else { 2637.0 };
            let sq = ((t * f * 6.28318).sin().signum()) * 0.4;
            let s = sq * env(t, dur, 0.002, 0.1) * 0.6;
            out.push(s);
        }
        write_wav(&dir.join("coin.wav"), &out);
    }
    // шаги (2 варианта)
    for (name, seed) in [("step1", 11u32), ("step2", 22u32)] {
        let dur = 0.12;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(seed);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let thump = (t * 80.0 * 6.28318).sin() * 0.7;
            let scuff = noise.next() * 0.25;
            let s = (thump + scuff) * env(t, dur, 0.002, 0.08) * 0.5;
            out.push(s);
        }
        write_wav(&dir.join(format!("{name}.wav")), &out);
    }
    // скрип двери
    {
        let dur = 0.5;
        let n = (SR as f32 * dur) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let f = 300.0 - t * 250.0 + (t * 31.0).sin() * 30.0;
            let s = ((t * f * 6.28318).sin()) * env(t, dur, 0.05, 0.3) * 0.4;
            out.push(s);
        }
        write_wav(&dir.join("door.wav"), &out);
    }
    // свисток милиционера
    {
        let dur = 0.5;
        let n = (SR as f32 * dur) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let trem = 1.0 + (t * 30.0).sin() * 0.3;
            let sq = ((t * 1400.0 * 6.28318).sin().signum()) * 0.4 * trem;
            let s = sq * env(t, dur, 0.02, 0.15) * 0.6;
            out.push(s);
        }
        write_wav(&dir.join("whistle.wav"), &out);
    }
    // «поп» — откупорка
    {
        let dur = 0.15;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(808);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let click = noise.next() * (-(t * 60.0)).exp();
            let tone = (t * (400.0 - t * 1500.0) * 6.28318).sin() * (-(t * 30.0)).exp();
            let s = (click * 0.6 + tone * 0.4) * 0.8;
            out.push(s);
        }
        write_wav(&dir.join("pop.wav"), &out);
    }
    // бурление бочки (петля)
    {
        let dur = 2.0;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(6060);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            lp = lp * 0.8 + noise.next() * 0.2;
            let wob = (t * 0.9).sin() * 0.5 + 0.5;
            let bubble = if ((t * 7.3).fract()) < 0.12 { 0.4 } else { 0.0 };
            let s = (lp * 0.3 + bubble * (t * 140.0 * 6.28318).sin() * 0.4) * (0.4 + wob * 0.6) * 0.5;
            out.push(s);
        }
        write_wav(&dir.join("bubble_loop.wav"), &out);
    }
    // улица: летний гул + насекомые (петля)
    {
        let dur = 3.0;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(2024);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            lp = lp * 0.95 + noise.next() * 0.05;
            // стрёкот: короткие чирки 4 кГц
            let chirp_phase = (t * 2.7).fract();
            let cricket = if chirp_phase < 0.06 {
                ((t * 4000.0 * 6.28318).sin()) * 0.25
            } else {
                0.0
            };
            let s = lp * 0.5 + cricket;
            out.push(s);
        }
        write_wav(&dir.join("street_loop.wav"), &out);
    }
    // «тревога» (ТЗ №5) — бандиты на подходе: два глухих удара + низкий скрип
    {
        let dur = 1.6;
        let n = (SR as f32 * dur) as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            // глухие удары (как кулак по кассе) в начале и через полсекунды
            let mut thump = 0.0;
            for &hit in [0.0, 0.55].iter() {
                let d = t - hit;
                if (0.0..0.4).contains(&d) {
                    let f = 70.0 - d * 60.0; // провал вниз тона
                    thump += (d * f * 6.28318).sin() * (1.0 - d / 0.4).powi(2) * 0.9;
                }
            }
            // низкий скрип струны — «чувство опасности»
            let scrape = ((t * 150.0).sin() + (t * 151.7).sin() * 0.7)
                * 0.12
                * (t / dur).powf(0.5).min(1.0);
            out.push((thump + scrape) * 0.8);
        }
        write_wav(&dir.join("alarm.wav"), &out);
    }
    // «Заводской гудок» (день 3, час пик): протяжный низкий гул паровоза.
    // Три тона с расстройкой (эффект «хорового» гудка) + паровой шум.
    {
        let dur = 3.6;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(31337);
        let mut lp = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            // лёгкая вибрация «живого» пара
            let wob = 1.0 + (t * 4.6).sin() * 0.004;
            let mut tone = 0.0;
            for (f, a) in [(92.0f32, 1.0f32), (116.0, 0.7), (184.0, 0.35), (232.0, 0.2)] {
                tone += (t * f * wob * 6.28318).sin() * a;
            }
            lp = lp * 0.92 + noise.next() * 0.08;
            let steam = lp * 0.18 * (1.0 - (t / dur)).max(0.0);
            let s = (tone * 0.30 + steam) * env(t, dur, 0.35, 1.4) * 0.9;
            out.push(s);
        }
        write_wav(&dir.join("horn.wav"), &out);
    }
    // «стинг» сцены бандита: резкий удар, когда он выбегает из-за угла
    {
        let dur = 1.5;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(41);
        let mut lp = 0.0f32;
        let mut p1 = 0.0f32;
        let mut p2 = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let f = 120.0 + 360.0 * (-t * 7.0).exp();
            p1 = (p1 + f / SR as f32).fract();
            p2 = (p2 + f * 1.06 / SR as f32).fract();
            let saw = (p1 * 2.0 - 1.0) + (p2 * 2.0 - 1.0);
            lp = lp * 0.72 + saw * 0.5 * 0.28;
            let e = (t / 0.004).min(1.0) * (-t * 2.2).exp();
            let sub = (t * (52.0 - t * 6.0) * 6.28318).sin() * (-t * 2.8).exp() * 0.7;
            let metal = (t * 1830.0 * 6.28318).sin() * (-t * 7.0).exp() * 0.16
                + (t * 2710.0 * 6.28318).sin() * (-t * 9.0).exp() * 0.10;
            let crack = noise.next() * (-t * 45.0).exp() * 0.45;
            out.push(lp * e * 0.9 + sub + metal + crack);
        }
        write_wav(&dir.join("sting.wav"), &out);
    }
    // нагнетающая петля под разговор с бандитом (8 с, бесшовно)
    {
        use std::f32::consts::TAU;
        let dur = 8.0;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(7);
        let mut p55 = 0.0f32;
        let mut p110 = 0.0f32;
        let mut p165 = 0.0f32;
        let mut p51 = 0.0f32;
        let mut p440 = 0.0f32;
        let mut p466 = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            p55 = (p55 + 55.0 / SR as f32).fract();
            p110 = (p110 + 110.0 / SR as f32).fract();
            p165 = (p165 + 165.0 / SR as f32).fract();
            p51 = (p51 + 51.0 / SR as f32).fract();
            p440 = (p440 + 440.0 / SR as f32).fract();
            p466 = (p466 + 466.25 / SR as f32).fract();
            let breathe = 0.55 + 0.45 * (t * 0.25 * TAU - std::f32::consts::FRAC_PI_2).sin();
            let drone = ((p55 * TAU).sin() * 0.55
                + (p110 * TAU).sin() * 0.26
                + (p165 * TAU).sin() * 0.12
                + (p51 * TAU).sin() * 0.20)
                * breathe;
            let d = t % 1.0;
            let pulse = (d * 48.0 * TAU).sin() * (-d * 9.0).exp() * 0.42;
            let swell = 0.5 - 0.5 * (t / dur * TAU).cos();
            let trem = 0.7 + 0.3 * (t * 4.0 * TAU).sin();
            let strings = ((p440 * TAU).sin() + (p466 * TAU).sin()) * 0.075 * swell * trem;
            let dt = t % 0.5;
            let tick = noise.next() * (-dt * 55.0).exp() * 0.05;
            out.push(drone * 0.75 + pulse + strings + tick);
        }
        write_wav(&dir.join("tension_loop.wav"), &out);
    }
    // звук удара: низкий «бум» + хруст
    {
        let dur = 0.5;
        let n = (SR as f32 * dur) as usize;
        let mut noise = Noise(99);
        let mut ph = 0.0f32;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let f = 45.0 + 120.0 * (-t * 14.0).exp();
            ph = (ph + f / SR as f32).fract();
            let thump = (ph * 6.28318).sin() * (-t * 7.5).exp();
            let crack = noise.next() * (-t * 38.0).exp() * 0.55;
            let skin = (t * 760.0 * 6.28318).sin() * (-t * 24.0).exp() * 0.18;
            out.push(thump + crack + skin);
        }
        write_wav(&dir.join("punch.wav"), &out);
    }
}

/// Банк загруженных сэмплов.
#[derive(Resource)]
pub struct SoundBank {
    pub handles: HashMap<String, Handle<AudioSource>>,
    /// Встроенная музыка меню (FLAC в exe).
    pub music: Handle<AudioSource>,
    /// Встроенная музыка игрового дня (FLAC в exe).
    pub game_music: Handle<AudioSource>,
    /// Встроенная музыка итогов дня (FLAC в exe).
    pub day_end_music: Handle<AudioSource>,
    pub step_toggle: bool,
    pub step_t: f32,
    pub last_customers: usize,
    pub ambience_on: bool,
}

/// Все SFX встроены в exe: релизный бинарник работает без папки assets.
fn sfx_bytes(name: &str) -> &'static [u8] {
    match name {
        "pour" => include_bytes!("../assets/sounds/pour.wav"),
        "croutons" => include_bytes!("../assets/sounds/croutons.wav"),
        "water" => include_bytes!("../assets/sounds/water.wav"),
        "hiss" => include_bytes!("../assets/sounds/hiss.wav"),
        "ding" => include_bytes!("../assets/sounds/ding.wav"),
        "hiss2" => include_bytes!("../assets/sounds/hiss2.wav"),
        "fooff" => include_bytes!("../assets/sounds/fooff.wav"),
        "coin" => include_bytes!("../assets/sounds/coin.wav"),
        "step1" => include_bytes!("../assets/sounds/step1.wav"),
        "step2" => include_bytes!("../assets/sounds/step2.wav"),
        "door" => include_bytes!("../assets/sounds/door.wav"),
        "whistle" => include_bytes!("../assets/sounds/whistle.wav"),
        "pop" => include_bytes!("../assets/sounds/pop.wav"),
        "bubble_loop" => include_bytes!("../assets/sounds/bubble_loop.wav"),
        "street_loop" => include_bytes!("../assets/sounds/street_loop.wav"),
        "alarm" => include_bytes!("../assets/sounds/alarm.wav"),
        "horn" => include_bytes!("../assets/sounds/horn.wav"),
        "sting" => include_bytes!("../assets/sounds/sting.wav"),
        "tension_loop" => include_bytes!("../assets/sounds/tension_loop.wav"),
        "punch" => include_bytes!("../assets/sounds/punch.wav"),
        _ => &[],
    }
}

pub fn load_bank(mut commands: Commands, mut assets: ResMut<Assets<AudioSource>>) {
    let mut handles = HashMap::new();
    for name in SOUNDS {
        let bytes = sfx_bytes(name);
        if bytes.is_empty() {
            continue;
        }
        let h = assets.add(AudioSource {
            bytes: Arc::from(bytes),
        });
        handles.insert(name.to_string(), h);
    }
    // Музыка меню — не через AssetServer (не нужен файл на диске):
    // кладём байты FLAC прямо в ассеты (Bevy 0.20: AudioSource { bytes }).
    let music = assets.add(AudioSource {
        bytes: Arc::from(MENU_MUSIC),
    });
    let game_music = assets.add(AudioSource {
        bytes: Arc::from(GAME_MUSIC),
    });
    let day_end_music = assets.add(AudioSource {
        bytes: Arc::from(DAY_END_MUSIC),
    });
    commands.insert_resource(MusicDuck(1.0));
    commands.insert_resource(SoundBank {
        handles,
        music,
        game_music,
        day_end_music,
        step_toggle: false,
        step_t: 0.0,
        last_customers: 0,
        ambience_on: false,
    });
}

/// Играет сэмпл (одноразово).
pub fn play_sfx(commands: &mut Commands, bank: &SoundBank, name: &str, volume: f32) {
    if let Some(h) = bank.handles.get(name) {
        commands.spawn((
            AudioPlayer::new(h.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::Linear(volume),
                ..default()
            },
        ));
    }
}

/// Звуки сцены бандита: они звучат, даже когда мир заморожен.
#[derive(Component)]
pub struct AmbushSound;

/// Петля нагнетания в сцене бандита.
#[derive(Component)]
pub struct TensionLoop;

fn play_ambush_sfx(commands: &mut Commands, bank: &SoundBank, name: &str, volume: f32) {
    if let Some(h) = bank.handles.get(name) {
        commands.spawn((
            AudioPlayer::new(h.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::Linear(volume),
                ..default()
            },
            AmbushSound,
        ));
    }
}

/// Сцена бандита: резкий стинг на выходе, нагнетающая петля под разговор
/// и глухой удар, если он бьёт. В отличие от остального звука, эти сэмплы
/// не замирают вместе с миром (`ambush.freeze()`).
pub fn ambush_sound(
    time: Res<Time>,
    mut commands: Commands,
    bank: Res<SoundBank>,
    ambush: Res<crate::ambush::BanditAmbush>,
    mut loops: Query<&mut AudioSink, With<TensionLoop>>,
    running: Query<Entity, With<TensionLoop>>,
    mut was_active: Local<bool>,
    mut was_hit: Local<bool>,
    mut fade: Local<f32>,
) {
    let active = ambush.freeze();
    if active && !*was_active {
        play_ambush_sfx(&mut commands, &bank, "sting", 0.85);
        if let Some(h) = bank.handles.get("tension_loop") {
            commands.spawn((
                AudioPlayer::new(h.clone()),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::Linear(0.0),
                    ..default()
                },
                TensionLoop,
                AmbushSound,
            ));
        }
        *fade = 0.0;
    }
    // Нагнетание проявляется после стинга и гаснет вместе со сценой.
    let target = if active { 1.0 } else { 0.0 };
    *fade += (target - *fade) * (time.delta_secs() * 1.8).min(1.0);
    for mut sink in &mut loops {
        sink.set_volume(bevy::audio::Volume::Linear(0.34 * *fade));
    }
    if !active && *was_active {
        for e in &running {
            commands.entity(e).despawn();
        }
        *fade = 0.0;
        *was_hit = false;
    }
    // Удар — ровно один раз на попадание (флаг живёт и после закрытия окна).
    if active && ambush.black_eye_hit && !*was_hit {
        play_ambush_sfx(&mut commands, &bank, "punch", 0.95);
    }
    *was_hit = ambush.black_eye_hit;
    *was_active = active;
}

/// Маркеры петель-фонов (бурление бочки).
#[derive(Component)]
pub struct LoopMark(pub &'static str);

/// Фон игры: только бурление бочки. Уличный шум убран — под музыку он
/// читался как «какой-то шум»; логичные эффекты (касса, наливание,
/// клиенты, тревога) остаются.
pub fn ambience(
    mut commands: Commands,
    bank: Res<SoundBank>,
    state: Res<GameState>,
    mut was_playing: Local<bool>,
    loops: Query<(Entity, &LoopMark)>,
) {
    let playing = state.phase == Phase::Playing;
    if playing && !*was_playing {
        if let Some(h) = bank.handles.get("bubble_loop") {
            commands.spawn((
                AudioPlayer::new(h.clone()),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::Linear(0.0),
                    ..default()
                },
                LoopMark("bubble"),
            ));
        }
    }
    if !playing && *was_playing {
        for (e, _) in &loops {
            commands.entity(e).despawn();
        }
    }
    *was_playing = playing;
}

/// Какой трек должен звучать в текущей фазе.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Menu,
    Game,
    /// Отчёт в конце дня («итоги смены»).
    DayEnd,
}

/// Длительность кроссфейда между треками (сек): переход слышен,
/// но не тянется — смена настроения, а не резкий обрыв.
const MUSIC_FADE: f32 = 0.8;

fn wanted_track(phase: Phase) -> Option<Track> {
    match phase {
        Phase::Menu | Phase::Story => Some(Track::Menu),
        Phase::Tutorial | Phase::Playing | Phase::Paused => Some(Track::Game),
        Phase::DayEnd => Some(Track::DayEnd),
        Phase::Final => None,
    }
}

/// Маркер звучащей музыки: трек, базовая громкость и коэффициент кроссфейда.
#[derive(Component)]
pub struct MusicMark {
    pub track: Track,
    pub base: f32,
    /// 0 — тишина, 1 — полная громкость (плавно меняется при смене трека).
    pub gain: f32,
}

impl MusicMark {
    fn new(track: Track) -> Self {
        // Игровая тема намеренно тише меню: она фон, а не солист —
        // эффекты (касса, наливание, клиенты) должны быть слышны.
        let base = match track {
            Track::Menu => 0.40,
            Track::Game => 0.22,
            Track::DayEnd => 0.30,
        };
        Self {
            track,
            base,
            gain: 0.0,
        }
    }
}

/// Музыка: тема меню, тема игры и тема итогов дня. Все треки зациклены,
/// смена фазы запускает новый трек с кроссфейдом (старый плавно уходит).
pub fn music(
    mut commands: Commands,
    bank: Res<SoundBank>,
    state: Res<GameState>,
    players: Query<(Entity, &MusicMark)>,
) {
    let Some(track) = wanted_track(state.phase) else {
        return;
    };
    // нужный трек уже звучит (в т.ч. если он ещё плавно проявляется)
    if players.iter().any(|(_, m)| m.track == track) {
        return;
    }
    let h = match track {
        Track::Menu => bank.music.clone(),
        Track::Game => bank.game_music.clone(),
        Track::DayEnd => bank.day_end_music.clone(),
    };
    commands.spawn((
        AudioPlayer::new(h),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            // стартуем с нуля: громкость даст кроссфейд
            volume: bevy::audio::Volume::Linear(0.0),
            ..default()
        },
        MusicMark::new(track),
    ));
}

/// Текущий коэффициент приглушения музыки (1.0 — полная громкость).
#[derive(Resource)]
pub struct MusicDuck(pub f32);

/// Кроссфейд треков, приглушение под эффекты и мастер-громкость настроек.
pub fn music_volume(
    time: Res<Time>,
    gvol: Res<GlobalVolume>,
    state: Res<GameState>,
    mut commands: Commands,
    mut duck: ResMut<MusicDuck>,
    mut music: Query<(Entity, &mut MusicMark, &mut AudioSink)>,
    sfx: Query<&AudioSink, (Without<MusicMark>, Without<LoopMark>)>,
) {
    let dt = time.delta_secs();
    let wanted = wanted_track(state.phase);
    let busy = sfx.iter().any(|s| !s.empty());
    let target = if busy { 0.55 } else { 1.0 };
    duck.0 += (target - duck.0) * (dt * 6.0).min(1.0);
    let g = gvol.volume.to_linear();
    let step = dt / MUSIC_FADE;
    for (e, mut mark, mut sink) in &mut music {
        let keep = wanted == Some(mark.track);
        if keep {
            mark.gain = (mark.gain + step).min(1.0);
        } else {
            mark.gain = (mark.gain - step).max(0.0);
        }
        sink.set_volume(Volume::Linear(mark.base * mark.gain * duck.0 * g));
        // старый трек полностью ушёл — убираем
        if !keep && mark.gain <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

/// Громкость бурления — тише, когда игрок далеко от бочки.
pub fn bubble_volume(
    cam: Res<crate::raycaster::GameCam>,
    barrels: Res<crate::brewing::Barrels>,
    mut loops: Query<(&LoopMark, &mut AudioSink)>,
) {
    let mut dist = f32::MAX;
    for &(x, y) in barrels.map.keys() {
        let d = ((x as f32 + 0.5 - cam.x).powi(2) + (y as f32 + 0.5 - cam.y).powi(2)).sqrt();
        dist = dist.min(d);
    }
    let vol = (1.0 - dist / 6.0).clamp(0.0, 1.0) * 0.5;
    for (mark, mut sink) in &mut loops {
        if mark.0 == "bubble" {
            sink.set_volume(bevy::audio::Volume::Linear(vol));
        }
    }
}

/// Шаги при ходьбе.
pub fn footsteps(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<GameState>,
    mut bank: ResMut<SoundBank>,
    mut commands: Commands,
) {
    if state.phase != Phase::Playing {
        return;
    }
    let moving = keys.pressed(KeyCode::KeyW)
        || keys.pressed(KeyCode::KeyS)
        || keys.pressed(KeyCode::KeyA)
        || keys.pressed(KeyCode::KeyD)
        || keys.pressed(KeyCode::ArrowUp)
        || keys.pressed(KeyCode::ArrowDown);
    if !moving {
        bank.step_t = 0.0;
        return;
    }
    bank.step_t -= time.delta_secs();
    if bank.step_t <= 0.0 {
        bank.step_t = 0.38;
        bank.step_toggle = !bank.step_toggle;
        let name = if bank.step_toggle { "step1" } else { "step2" };
        play_sfx(&mut commands, &bank, name, 0.3);
    }
}

/// Звонки при сигналах бочки + дверь при новых покупателях.
pub fn event_sounds(
    mut commands: Commands,
    mut barrels: ResMut<crate::brewing::Barrels>,
    customers: Query<&crate::customer::Customer>,
    mut bank: ResMut<SoundBank>,
) {
    for b in barrels.map.values_mut() {
        if let Some(sig) = b.signal.take() {
            use crate::brewing::BarrelSignal::*;
            let name = match sig {
                AlmostReady => "hiss",
                Ready => "ding",
                OverripeStart => "hiss2",
                Ruined => "fooff",
            };
            play_sfx(&mut commands, &bank, name, 0.7);
        }
    }
    let n = customers.iter().count();
    if n > bank.last_customers {
        play_sfx(&mut commands, &bank, "door", 0.5);
    }
    bank.last_customers = n;
}

/// Полная пауза звука, пока открыта отладочная консоль или идёт сцена
/// бандита. Музыка и уже запущенные SFX/петли замирают на месте; после
/// закрытия продолжают с той же точки. Новые звуки в это время не спавнятся
/// (игровые аудиосистемы гейтятся в main.rs), но проверка каждого sink
/// защищает и от «догоняющих» звуков предыдущего кадра.
pub fn pause_for_overlays(
    debug: Res<crate::debug::DebugConsole>,
    ambush: Res<crate::ambush::BanditAmbush>,
    sinks: Query<(&AudioSink, Option<&AmbushSound>)>,
) {
    for (sink, ambush_sound) in &sinks {
        // Звуки сцены бандита живут вместе с замороженным миром, но
        // отладочная консоль глушит всё, включая их.
        let should_pause = debug.open || (ambush.freeze() && ambush_sound.is_none());
        if should_pause {
            if !sink.is_paused() {
                sink.pause();
            }
        } else if sink.is_paused() {
            sink.play();
        }
    }
}
