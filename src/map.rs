//! 2D-карта лавки (тайлы 1x1 юнит), как в Wolf3D/Doom.

use crate::textures::TexId;

pub const MAP_W: usize = 18;
pub const MAP_H: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// Проходимый пол.
    Floor,
    /// Кафельная стена.
    Wall,
    /// Прилавок (непроходимый).
    Counter,
    /// Бочка с краном (непроходимая станция).
    Barrel,
    /// Стеллаж с посудой.
    Shelf,
    /// Полка с сухарями.
    Croutons,
    /// Банка сахара/изюма.
    Sugar,
    /// Колонка с водой.
    Water,
    /// Окно снабжения (закупка товара, ТЗ №3).
    Supply,
    /// Мойка посуды (мини-игра протирания губкой, ТЗ №4).
    Sink,
    /// Дверной проём (проходимый, видна улица за ним).
    Doorway,
}

impl Tile {
    pub fn solid(self) -> bool {
        !matches!(self, Tile::Floor | Tile::Doorway)
    }

    /// Блокирует ли тайл луч взгляда (для рендера).
    pub fn ray_solid(self) -> bool {
        self.solid() || self == Tile::Doorway
    }

    /// Верхняя текстура (для стен/предметов).
    pub fn tex(self) -> Option<TexId> {
        Some(match self {
            Tile::Wall => TexId::TileWall,
            Tile::Counter => TexId::Counter,
            Tile::Barrel => TexId::Barrel,
            Tile::Shelf => TexId::Shelf,
            Tile::Croutons => TexId::CroutonsShelf,
            Tile::Sugar => TexId::SugarShelf,
            Tile::Water => TexId::WaterTap,
            Tile::Supply => TexId::Supply,
            Tile::Sink => TexId::Sink,
            Tile::Doorway => TexId::Outside,
            Tile::Floor => return None,
        })
    }

    /// Текстура верхней грани низкой мебели (её видно сверху под углом).
    /// Фасадная текстура для крышки не годится: растянутая по глубине,
    /// она превращала рисунок кассы в мутные полосы.
    pub fn top_tex(self) -> Option<TexId> {
        match self {
            Tile::Counter => Some(TexId::CounterTop),
            Tile::Barrel => Some(TexId::BarrelTop),
            _ => None,
        }
    }
}

#[derive(bevy::prelude::Resource, Clone)]
pub struct Map {
    pub tiles: Vec<Tile>,
}

impl Map {
    /// Лавка на две комнаты (план 2026-10-22): торговый зал + склад,
    /// соединённые коридором в юго-восточном углу зала.
    ///
    /// ```text
    /// #....DD..#########   D — вход (5,0)(6,0), DOOR=(5.5,0.5)
    /// #........#########   очередь x=5.5 вниз к прилавку (3.5/2.5/1.5)
    /// #........#########
    /// #........#########
    /// #...CCCCC#####AAA#   C — прилавок: Г-образный, (4..8,4)+(4,5..6)
    /// #...C...M#####...#   M — мойка (8,5)
    /// #P..C...........W#   P — заказы/снабжение (1,6)
    /// #........#####...#   коридор y=6: x=9..13; склад x=14..16, y=4..8
    /// #.....BSS#####UUU#   B — бочка (6,8), S — посуда (7..8,8),
    ///                     A — сухари (север), U — сахар (юг), W — вода
    /// ##################   мусорка — спрайт на полу (5.5,8.3)
    /// ```
    pub fn shop() -> Self {
        // Весь мир — стены; комнаты вырезаем полом.
        let mut m = Self {
            tiles: vec![Tile::Wall; MAP_W * MAP_H],
        };
        // Торговый зал: x=1..8, y=1..8.
        for y in 1..=8 {
            for x in 1..=8 {
                m.set(x, y, Tile::Floor);
            }
        }
        // Склад: x=14..16, y=4..8 (стены комнаты — x=13/17, y=3/9).
        for y in 4..=8 {
            for x in 14..=16 {
                m.set(x, y, Tile::Floor);
            }
        }
        // Коридор зал → склад: один тайл высотой, y=6.
        for x in 10..=12 {
            m.set(x, 6, Tile::Floor);
        }
        m.set(9, 6, Tile::Floor); // проём в восточной стене зала
        m.set(13, 6, Tile::Floor); // проём в западной стене склада
        // Вход в зал — два тайла в северной стене над очередью.
        m.set(5, 0, Tile::Doorway);
        m.set(6, 0, Tile::Doorway);
        // Прилавок: горизонталь вдоль зала + вертикаль вниз у левого конца.
        for x in 4..=8 {
            m.set(x, 4, Tile::Counter);
        }
        m.set(4, 5, Tile::Counter);
        m.set(4, 6, Tile::Counter);
        // Мойка — у правого конца прилавка.
        m.set(8, 5, Tile::Sink);
        // Заказы/окно снабжения — в левой стене зала.
        m.set(1, 6, Tile::Supply);
        // Южная стена зала: посуда, а между ней и мусоркой — бочка.
        m.set(6, 8, Tile::Barrel);
        for x in 7..=8 {
            m.set(x, 8, Tile::Shelf);
        }
        // Склад: сухари — северная стена, сахар — южная, вода — восточная.
        for x in 14..=16 {
            m.set(x, 4, Tile::Croutons);
            m.set(x, 8, Tile::Sugar);
        }
        m.set(16, 6, Tile::Water);
        m
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Tile {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return Tile::Wall;
        }
        self.tiles[y as usize * MAP_W + x as usize]
    }

    fn set(&mut self, x: usize, y: usize, t: Tile) {
        self.tiles[y * MAP_W + x] = t;
    }
}
