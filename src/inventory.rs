//! Инвентарь: руки (4 слота) + склад (закупка сырья).

use bevy::prelude::*;

use crate::sprites::Vessel;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HandItem {
    /// Горсть сухарей (1..3 в слоте).
    Croutons(u8),
    /// Ведро воды.
    Bucket,
    /// Порция сахара с изюмом (1..3).
    Sugar(u8),
    /// Пустая посуда (1..4).
    Empty(Vessel, u8),
    /// Полная посуда. bool — квас перебродил.
    Full(Vessel, u8, bool),
}

impl HandItem {
    pub fn is_full(&self, v: Vessel) -> bool {
        matches!(self, HandItem::Full(w, _, _) if *w == v)
    }
}

#[derive(Resource)]
pub struct Inventory {
    pub hands: [Option<HandItem>; 4],
    pub selected: usize,
    /// Склад: сколько «порций» сырья куплено и осталось.
    pub stock_croutons: u8,
    pub stock_sugar: u8,
    /// Стаканы — многоразовые (ТЗ №8): пул = stock_glass + dirty_glasses
    /// постоянно, после питья стакан становится грязным, мойка возвращает
    /// его на склад. Покупать стаканы нельзя.
    pub stock_glass: u8,
    /// Грязные стаканы, ждущие мойки (на краю прилавка/складе).
    pub dirty_glasses: u8,
    pub stock_jar: u8,
    pub stock_bottle: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            hands: [None, None, None, None],
            selected: 0,
            stock_croutons: 0,
            stock_sugar: 0,
            stock_glass: 0,
            dirty_glasses: 0,
            stock_jar: 0,
            stock_bottle: 0,
        }
    }
}

impl Inventory {
    pub fn is_empty(&self) -> bool {
        self.hands.iter().all(|h| h.is_none())
    }

    /// Кладёт предмет в первый подходящий слот. false — нет места.
    pub fn put(&mut self, item: HandItem) -> bool {
        // стак в тот же тип
        for slot in self.hands.iter_mut() {
            match (slot, &item) {
                (Some(HandItem::Croutons(a)), HandItem::Croutons(b)) if *a + b <= 3 => {
                    *a += b;
                    return true;
                }
                (Some(HandItem::Sugar(a)), HandItem::Sugar(b)) if *a + b <= 3 => {
                    *a += b;
                    return true;
                }
                (Some(HandItem::Empty(va, a)), HandItem::Empty(vb, b))
                    if va == vb && *a + b <= 4 =>
                {
                    *a += b;
                    return true;
                }
                (Some(HandItem::Full(va, a, oa)), HandItem::Full(vb, b, ob))
                    if va == vb && oa == ob && *a + b <= 4 =>
                {
                    *a += b;
                    return true;
                }
                _ => {}
            }
        }
        for slot in self.hands.iter_mut() {
            if slot.is_none() {
                *slot = Some(item);
                return true;
            }
        }
        false
    }

    /// Забирает одну единицу из слота selected (для частичного расхода).
    pub fn take_one_from_selected(&mut self) -> Option<HandItem> {
        let slot = &mut self.hands[self.selected];
        match slot {
            Some(HandItem::Croutons(n)) => {
                *n -= 1;
                if *n == 0 {
                    *slot = None;
                    return Some(HandItem::Croutons(1));
                }
                Some(HandItem::Croutons(1))
            }
            Some(HandItem::Sugar(n)) => {
                *n -= 1;
                if *n == 0 {
                    *slot = None;
                }
                Some(HandItem::Sugar(1))
            }
            Some(HandItem::Empty(v, n)) => {
                *n -= 1;
                let v = *v;
                if *n == 0 {
                    *slot = None;
                }
                Some(HandItem::Empty(v, 1))
            }
            Some(HandItem::Full(v, n, o)) => {
                *n -= 1;
                let (v, o) = (*v, *o);
                if *n == 0 {
                    *slot = None;
                }
                Some(HandItem::Full(v, 1, o))
            }
            _ => None,
        }
    }

    pub fn has_bucket(&self) -> bool {
        self.hands.iter().any(|h| matches!(h, Some(HandItem::Bucket)))
    }

    pub fn drain_bucket(&mut self) -> bool {
        for slot in self.hands.iter_mut() {
            if matches!(slot, Some(HandItem::Bucket)) {
                *slot = None;
                return true;
            }
        }
        false
    }

    pub fn has_croutons(&self) -> bool {
        self.hands.iter().any(|h| matches!(h, Some(HandItem::Croutons(_))))
    }

    pub fn drain_croutons(&mut self) -> bool {
        for slot in self.hands.iter_mut() {
            if let Some(HandItem::Croutons(n)) = slot {
                *n -= 1;
                let empty = *n == 0;
                if empty {
                    *slot = None;
                }
                return true;
            }
        }
        false
    }

    pub fn has_sugar(&self) -> bool {
        self.hands.iter().any(|h| matches!(h, Some(HandItem::Sugar(_))))
    }

    pub fn drain_sugar(&mut self) -> bool {
        for slot in self.hands.iter_mut() {
            if let Some(HandItem::Sugar(n)) = slot {
                *n -= 1;
                let empty = *n == 0;
                if empty {
                    *slot = None;
                }
                return true;
            }
        }
        false
    }

    /// Первый полный предмет нужного вида (для выдачи покупателю).
    pub fn take_full(&mut self, v: Vessel) -> Option<(Vessel, bool)> {
        for slot in self.hands.iter_mut() {
            if let Some(HandItem::Full(w, n, o)) = slot {
                if *w == v {
                    *n -= 1;
                    let (w, o) = (*w, *o);
                    if *n == 0 {
                        *slot = None;
                    }
                    return Some((w, o));
                }
            }
        }
        None
    }

    /// Взять пустую посуду со стеллажа (склад -> руки), циклом стакан/банка/бутылка.
    pub fn take_empty_from_stock(&mut self, cycle: &mut usize) -> Option<Vessel> {
        for _ in 0..3 {
            let idx = *cycle % 3;
            *cycle += 1;
            let got = match idx {
                0 if self.stock_glass > 0 => {
                    self.stock_glass -= 1;
                    Some(Vessel::Glass)
                }
                1 if self.stock_jar > 0 => {
                    self.stock_jar -= 1;
                    Some(Vessel::Jar)
                }
                2 if self.stock_bottle > 0 => {
                    self.stock_bottle -= 1;
                    Some(Vessel::Bottle)
                }
                _ => None,
            };
            if got.is_some() {
                return got;
            }
        }
        None
    }

    pub fn take_croutons_from_stock(&mut self) -> bool {
        if self.stock_croutons > 0 {
            self.stock_croutons -= 1;
            self.put(HandItem::Croutons(1))
        } else {
            false
        }
    }

    pub fn take_sugar_from_stock(&mut self) -> bool {
        if self.stock_sugar > 0 {
            self.stock_sugar -= 1;
            self.put(HandItem::Sugar(1))
        } else {
            false
        }
    }
}
