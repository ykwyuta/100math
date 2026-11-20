use crate::{axes, Operation, CELLS, SIDE};

pub const OPPONENTS: [&str; 12] = [
    "ミジンコ",
    "ありんこ",
    "ネズミ",
    "うさぎ",
    "赤ちゃん",
    "女の子",
    "灘高生",
    "東大生",
    "ヒュパティア",
    "ピタゴラス",
    "フェルマー",
    "ラマヌジャン",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Outcome {
    Playing,
    Win,
    Loss,
}

pub struct Battle {
    pub numbers: [i32; SIDE * 2],
    pub operation: Operation,
    pub player_hp: u32,
    pub opponent_hp: u32,
    pub opponent_max_hp: u32,
    pub combo: u32,
    pub cell: usize,
    pub attacks: u32,
    pub outcome: Outcome,
    interval: u32,
    damage: u32,
    next_attack: u32,
}

impl Battle {
    pub fn new(seed: u32, operation: Operation, opponent: usize) -> Self {
        let level = opponent.min(OPPONENTS.len() - 1) as u32;
        let interval = 8_000 - level * 420;
        let opponent_max_hp = 22 + level * 3;
        Self {
            numbers: axes(seed),
            operation,
            player_hp: 36,
            opponent_hp: opponent_max_hp,
            opponent_max_hp,
            combo: 0,
            cell: 0,
            attacks: 0,
            outcome: Outcome::Playing,
            interval,
            damage: 3 + level / 2,
            next_attack: interval,
        }
    }

    pub fn question(&self) -> (i32, i32) {
        (
            self.numbers[self.cell / SIDE],
            self.numbers[SIDE + self.cell % SIDE],
        )
    }

    pub fn remaining(&self, elapsed_ms: u32) -> u32 {
        self.next_attack.saturating_sub(elapsed_ms)
    }

    pub fn tick(&mut self, elapsed_ms: u32) {
        while self.outcome == Outcome::Playing && elapsed_ms >= self.next_attack {
            self.player_hp = self.player_hp.saturating_sub(self.damage);
            self.attacks += 1;
            self.next_attack += self.interval;
            if self.player_hp == 0 {
                self.outcome = Outcome::Loss;
            }
        }
    }

    pub fn submit(&mut self, answer: i32, elapsed_ms: u32) -> Option<bool> {
        self.tick(elapsed_ms);
        if self.outcome != Outcome::Playing {
            return None;
        }
        let (row, column) = self.question();
        let correct = answer == self.operation.answer(row, column);
        if correct {
            self.combo += 1;
            self.opponent_hp = self.opponent_hp.saturating_sub(2 + self.combo.min(5));
            if self.opponent_hp == 0 {
                self.outcome = Outcome::Win;
            } else {
                self.cell = (self.cell + 1) % CELLS;
            }
        } else {
            self.combo = 0;
        }
        Some(correct)
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static BATTLE: RefCell<Option<Battle>> = const { RefCell::new(None) };
    }

    #[no_mangle]
    pub extern "C" fn battle_start(seed: u32, operation: u32, opponent: u32) {
        BATTLE.with_borrow_mut(|slot| {
            *slot = Some(Battle::new(
                seed,
                Operation::from_index(operation),
                opponent as usize,
            ));
        });
    }

    #[no_mangle]
    pub extern "C" fn battle_tick(elapsed: u32) {
        BATTLE.with_borrow_mut(|slot| {
            if let Some(battle) = slot {
                battle.tick(elapsed);
            }
        });
    }

    #[no_mangle]
    pub extern "C" fn battle_submit(answer: i32, elapsed: u32) -> u32 {
        BATTLE.with_borrow_mut(
            |slot| match slot.as_mut().and_then(|b| b.submit(answer, elapsed)) {
                Some(true) => 2,
                Some(false) => 1,
                None => 0,
            },
        )
    }

    #[no_mangle]
    pub extern "C" fn battle_value(field: u32, elapsed: u32) -> u32 {
        BATTLE.with_borrow(|slot| {
            slot.as_ref().map_or(0, |b| match field {
                0 => b.player_hp,
                1 => b.opponent_hp,
                2 => b.opponent_max_hp,
                3 => b.combo,
                4 => b.cell as u32,
                5 => b.attacks,
                6 => b.outcome as u32,
                7 => b.remaining(elapsed),
                _ => 0,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_grows_resets_and_defeats_opponent() {
        let mut battle = Battle::new(42, Operation::Subtract, 0);
        let answer = |battle: &Battle| {
            let (row, col) = battle.question();
            row - col
        };
        assert_eq!(battle.submit(answer(&battle), 10), Some(true));
        assert_eq!(battle.submit(answer(&battle), 20), Some(true));
        assert_eq!((battle.combo, battle.cell, battle.opponent_hp), (2, 2, 15));
        assert_eq!(battle.submit(answer(&battle) + 1, 30), Some(false));
        assert_eq!((battle.combo, battle.cell), (0, 2));
        for _ in 0..10 {
            battle.submit(answer(&battle), 40);
        }
        assert_eq!(battle.outcome, Outcome::Win);
        assert_eq!(battle.submit(0, 1_000_000), None);
        assert_eq!(battle.player_hp, 36);
    }

    #[test]
    fn cpu_attacks_on_schedule_and_can_win() {
        let mut battle = Battle::new(42, Operation::Add, 11);
        battle.tick(3_379);
        assert_eq!(battle.player_hp, 36);
        battle.tick(6_760);
        assert_eq!((battle.player_hp, battle.attacks), (20, 2));
        assert_eq!(battle.submit(0, 20_000), None);
        assert_eq!(battle.outcome, Outcome::Loss);
    }
}
