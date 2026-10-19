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
    /// Мойка посуды (мини-грамота мытья стаканов, ТЗ №8).
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
    /// Компактная лавка 12x8 (не продуктовый рынок): дверь в северной стене,
    /// прилавок посередине, бочки колонной у западной стены (к каждой можно
    /// подойти с востока), станции у южной стены.
    ///
    /// ```text
    /// #....DD....#   D — дверь (5,0)(6,0)
    /// #..........#
    /// #S.........#   S — мойка (2,3); очередь y=2.7 (5.5 / 5.5,1.7 / 5.5,0.7)
    /// #..CCCCCC..#   прилавок x3..8, y3
    /// #B.........#   бочки (1,4) (1,5) (1,6) — подход с востока
    /// #B.........#
    /// #B.CCWSOS.S#   сухари 3-4, сахар 5, вода 6, окно снабжения 7,
    ///               посуда 8-9 (y=6)
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
        // Дверь в северной стене (вид на улицу).
        m.set(5, 0, Tile::Doorway);
        m.set(6, 0, Tile::Doorway);
        // Прилавок: очередь с севера, игрок за ним.
        for x in 3..=8 {
            m.set(x, 3, Tile::Counter);
        }
        // Бочки с кранами колонной у западной стены; подход с востока
        // свободен (2,4) (2,5) (2,6) — до любой бочки можно дойти.
        m.set(1, 4, Tile::Barrel);
        m.set(1, 5, Tile::Barrel);
        m.set(1, 6, Tile::Barrel);
        // Станции вдоль южной стены.
        m.set(3, 6, Tile::Croutons);
        m.set(4, 6, Tile::Croutons);
        m.set(5, 6, Tile::Sugar);
        m.set(6, 6, Tile::Water);
        m.set(8, 6, Tile::Shelf);
        m.set(9, 6, Tile::Shelf);
        // Окно снабжения (ТЗ №3) и мойка посуды (ТЗ №8).
        m.set(7, 6, Tile::Supply);
        m.set(2, 3, Tile::Sink);
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
