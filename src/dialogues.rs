//! Реплики покупателей (13+, советский юмор). ВСЕ персонажи общаются текстом.

use rand::prelude::*;

use crate::sprites::{NpcKind, Vessel};

/// Реплика заказа — содержит «тайный заказ», который надо понять.
pub fn order_line(kind: NpcKind, v: Vessel, rng: &mut StdRng) -> String {
    if matches!(
        kind,
        NpcKind::Militsioner | NpcKind::Hooligan | NpcKind::Sanepid
    ) {
        return official_order_line(kind, v);
    }
    let lines: &[&str] = match (kind, v) {
        // Бабушка
        (NpcKind::Grandma, Vessel::Glass) => &[
            "Дай, внучек, стаканчик квасика, холодненький...",
            "Стакан, но погуще, а то опять водица.",
            "Чай без сахара — это не квас. Дай нормальный!",
        ],
        (NpcKind::Grandma, Vessel::Jar) => &[
            "Баночку на вынос, на вечерок.",
            "Дай банку, внучек, домашних дел полно.",
        ],
        (NpcKind::Grandma, Vessel::Bottle) => &[
            "Бутылку домой возьму, деду на огород.",
            "Дай бутылку, заодно и ноги отдохнут.",
        ],
        // Рабочий
        (NpcKind::Worker, Vessel::Glass) => &[
            "Стакан квасу, и побыстрее — обед через пять!",
            "Э, давай стакан, а то жажда убивает.",
        ],
        (NpcKind::Worker, Vessel::Jar) => &[
            "Банку давай, на перекур хватит.",
            "Мне банку. Не каприз — привычка.",
        ],
        (NpcKind::Worker, Vessel::Bottle) => &[
            "15 копеек?! У Дяди Вани дешевле! ...Ладно, давай бутылку.",
            "Э, что квас-то бурдой пахнет? Ладно — бутылку на пробу!",
        ],
        // Мама с ребёнком
        (NpcKind::Mama, Vessel::Glass) => &[
            "Быстрее, пожалуйста, у нас автобус через пять минут!",
            "Стакан, только не лей через край — мы в белом!",
        ],
        (NpcKind::Mama, Vessel::Jar) => &[
            "Сынок, ты опять весь в квасе?! Нам банку с собой.",
            "Баночку на вынос, а то руки заняты.",
        ],
        (NpcKind::Mama, Vessel::Bottle) => &[
            "Бутылку домой, гостей ждём. Быстро, пожалуйста!",
            "Дайте бутылку, у меня ребёнок и автобус!",
        ],
        // Студент
        (NpcKind::Student, Vessel::Glass) => &[
            "Стакан, но со скидкой — я институт питаю мозгами.",
            "Дайте стакан. Сдачи не надо — я студент.",
        ],
        (NpcKind::Student, Vessel::Jar) => &[
            "Банку, пожалуйста — под котлеты по-киевски.",
            "Мне банку. Диплом сам себя не защитит.",
        ],
        (NpcKind::Student, Vessel::Bottle) => &[
            "Бутылку! Всё, баста — сессия позади.",
            "Дай бутылку, а то стипендия уже в прошлом веке.",
        ],
        // официальные персонажи обрабатываются ранним выходом выше
        (NpcKind::Militsioner, _) | (NpcKind::Hooligan, _) | (NpcKind::Sanepid, _) => {
            &[militsioner_line_static()]
        }
    };
    lines.choose(rng).map(|s| s.to_string()).unwrap_or_default()
}

/// Официальные персонажи: реплики проверки.
fn official_order_line(kind: NpcKind, _v: Vessel) -> String {
    match kind {
        NpcKind::Militsioner => militsioner_line_static().to_string(),
        NpcKind::Sanepid => sanepid_line_static().to_string(),
        _ => "Дай стакан, а то не выйдет дружить.".to_string(),
    }
}

fn militsioner_line_static() -> &'static str {
    "Проверка. Санитарная книжка и крышка на бочке. Баночку на проверку!"
}

fn sanepid_line_static() -> &'static str {
    "Паспорт бочки! Брожение вне графика — нарушение! Банку на анализ."
}

/// Благодарность после оплаты.
pub fn thanks_line(kind: NpcKind, overripe: bool, rng: &mut StdRng) -> String {
    if overripe {
        let lines: &[&str] = &[
            "Это что за кислятина?! Половину денег верни!",
            "Квас-то перебродил! Так и быть, в этот раз.",
            "Эх, барин, квас-то испорчен. Дешевле бы дал.",
        ];
        return lines.choose(rng).unwrap().to_string();
    }
    let lines: &[&str] = match kind {
        NpcKind::Grandma => &[
            "Вот это квас! Как у бабушки в деревне!",
            "Умница, внучек. Сдачи не надо.",
        ],
        NpcKind::Worker => &[
            "Вот так бы всегда! Молодец.",
            "Э, нормальный квас! Ещё бы бутылку на дом.",
        ],
        NpcKind::Mama => &[
            "Спасибо большое, нам пора!",
            "Сынок, скажи дяде спасибо. А то опять молчит.",
        ],
        NpcKind::Student => &[
            "Сдачи не надо, спасибо, я студент, мне хватит.",
            "Квас огонь! Оценю на пять с плюсом.",
        ],
        NpcKind::Hooligan => &["Ну наконец-то! Цены-то космические."],
        NpcKind::Militsioner => &["Следующий раз предупрежу. А пока — спасибо."],
        NpcKind::Sanepid => &["Паспорт бочки в порядке. На первый раз отпущу."],
    };
    lines.choose(rng).unwrap().to_string()
}

/// Недовольство: долго ждал / нет кваса.
pub fn angry_line(kind: NpcKind, rng: &mut StdRng) -> String {
    let lines: &[&str] = match kind {
        NpcKind::Grandma => &[
            "Всё, меня тут не ждут. Пойду в «Пищевик»!",
            "И нечего тут торговать-то. Хамство одно.",
        ],
        NpcKind::Worker => &["Ладно, сам в «Пищевик» схожу. Всё тут!"],
        NpcKind::Mama => &["Всё, автобус уходит! Нам тут не рады."],
        NpcKind::Student => &["Уйду в столовую. Там хоть компот есть."],
        NpcKind::Hooligan => &["Э, барин, а ну считай! Я же вижу — кваса нет."],
        NpcKind::Militsioner => &["Запишу в журнал. Поговорим в следующий раз."],
        NpcKind::Sanepid => &["Зафиксирую нарушение в акте."],
    };
    lines.choose(rng).unwrap().to_string()
}

/// Реплика хулигана, угрожающего бочке.
pub fn hooligan_threat(rng: &mut StdRng) -> &'static str {
    let lines = [
        "Э, барин, отпусти стакан, а то бочку проломаю!",
        "Цены космические! Давай за полцены или жалобу напишу!",
        "А ну угощай, а то стекла побью!",
    ];
    lines.choose(rng).unwrap()
}

/// Реплика милиционера: проверка, взятка банкой.
pub fn militsioner_line(rng: &mut StdRng) -> &'static str {
    let lines = [
        "Проверка. Санитарная книжка и крышка на бочке.",
        "А где это бочка паспорт имеет? На проверку баночку!",
        "Не шурши — по закону спрашиваю. Баночку на пробу.",
    ];
    lines.choose(rng).unwrap()
}

/// Реплика санэпидемнадзора.
pub fn sanepid_line(rng: &mut StdRng) -> &'static str {
    let lines = [
        "А где тут паспорт бочки? Брожение вне графика — нарушение!",
        "Флажок срока годности висит? Нет? Это штраф.",
        "Грязь на полу — акт. Либо банка, либо 30 копеек.",
    ];
    lines.choose(rng).unwrap()
}

/// Покупатель ушёл недовольным — реплика на уходе.
pub fn leaving_line(kind: NpcKind, rng: &mut StdRng) -> &'static str {
    let lines: &[&str] = match kind {
        NpcKind::Grandma => &["И нечего тут торговать-то. Хамство одно."],
        NpcKind::Worker => &["Эх... Всё, пойду в «Пищевик»."],
        NpcKind::Mama => &["Всё, меня тут не ждут!"],
        NpcKind::Student => &["Угу. Ну его."],
        _ => &["До свидания. Больше не приду."],
    };
    lines.choose(rng).unwrap()
}

/// Подсказка новичку (день 1).
pub fn tutorial_lines(step: usize) -> &'static str {
    const L: &[&str] = &[
        "ДОБРО ПОЖАЛОВАТЬ! Возьмите сухари с полки (E).",
        "Теперь ведро воды у колонки (E).",
        "Сахар с изюмом на полке (E). Подойдите к бочке и жмите E трижды.",
        "Бочка бродит! Разливайте готовый квас в посуду со стеллажа.",
        "Покупатель ждёт за прилавком — подойдите с полной посудой и жмите E.",
    ];
    L.get(step).copied().unwrap_or("")
}
