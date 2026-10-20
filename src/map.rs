//! 2D-карта лавки (тайлы 1x1 юнит), как в Wolf3D/Doom.

use crate::textures::TexId;

pub const MAP_W: usize = 12;
pub const MAP_H: usize = 8;

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
}

#[derive(bevy::prelude::Resource, Clone)]
pub struct Map {
    pub tiles: Vec<Tile>,
}

impl Map {
    /// Компактная лавка 12x8 (ТЗ redesign): ЕДНА касса у правой стены,
    /// ЕДНА бочка у левой, дверь и очередь — колонной у правой стены,
    /// склад утоплен в заднюю стену — центр пола свободен для бега.
    ///
    /// ```text
    /// #.......DD.#   D — дверь (8,0)(9,0), над колонной очереди
    /// #..........#
    /// #.........#.   очередь x=9.5 (0.7 / 1.7 / 2.7 у кассы)
    /// #........C.#   касса (9,3): игрок с юга (отступ 0.6), клиент с севера
    /// #M.........#   M — мойка (1,4); урна-мусорка — спрайт на полу (2.3,4.3)
    /// #B.........#   бочка (1,5), подход с востока (2,5)
    /// #..CCAPO.W.#   сухари 3-4, сахар 5, посуда 6, снабжение 7,
    ///               вода+ведро 9 (y=6, задняя стена)
    /// #..........#   южная стена y=7
    /// ```
    pub fn shop() -> Self {
        let tiles = vec![Tile::Floor; MAP_W * MAP_H];
        let mut m = Self { tiles };
        // Внешние стены.
        for x in 0..MAP_W {
            m.set(x, 0, Tile::Wall);
            m.set(x, MAP_H - 1, Tile::Wall);
        }
        for y in 0..MAP_H {
            m.set(0, y, Tile::Wall);
            m.set(MAP_W - 1, y, Tile::Wall);
        }
        // Дверь в северной стене — прямо над колонной очереди.
        m.set(8, 0, Tile::Doorway);
        m.set(9, 0, Tile::Doorway);
        // ЕДИНСТВЕННАЯ касса у правой стены: игрок за ней с юга
        // (отступ 0.6 — ТЗ про дыру в прилавке), очередь к двери.
        m.set(9, 3, Tile::Counter);
        // ЕДИНСТВЕННАЯ бочка у левой стены, мойка посуды рядом.
        m.set(1, 5, Tile::Barrel);
        m.set(1, 4, Tile::Sink);
        // Складские станции утоплены в заднюю (южную) стену y=6,
        // центр пола свободен для бега (ТЗ: redesign планировки).
        m.set(3, 6, Tile::Croutons);
        m.set(4, 6, Tile::Croutons);
        m.set(5, 6, Tile::Sugar);
        m.set(6, 6, Tile::Shelf);
        m.set(7, 6, Tile::Supply);
        // Ведро с водой — правый угол за спиной кассира.
        m.set(9, 6, Tile::Water);
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
