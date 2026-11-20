use crate::collection::POINTS_PER_WIN;
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
    pub last_lines: u32,
    pub solved_count: u32,
    solved: u128,
    last_correct: Option<usize>,
    pub outcome: Outcome,
    interval: u32,
    damage: u32,
    next_attack: u32,
    reward_claimed: bool,
}

impl Battle {
    pub fn new(seed: u32, operation: Operation, opponent: usize) -> Self {
        let level = opponent.min(OPPONENTS.len() - 1) as u32;
        let interval = 9_000 - level * 300;
        let opponent_max_hp = Self::opponent_health(opponent);
        Self {
            numbers: axes(seed),
            operation,
            player_hp: 36,
            opponent_hp: opponent_max_hp,
            opponent_max_hp,
            combo: 0,
            cell: 0,
            attacks: 0,
            last_lines: 0,
            solved_count: 0,
            solved: 0,
            last_correct: None,
            outcome: Outcome::Playing,
            interval,
            damage: 3 + level / 3,
            next_attack: interval,
            reward_claimed: false,
        }
    }

    pub fn opponent_health(opponent: usize) -> u32 {
        46 + opponent.min(OPPONENTS.len() - 1) as u32 * 4
    }

    pub fn is_solved(&self, cell: usize) -> bool {
        cell < CELLS && self.solved & (1u128 << cell) != 0
    }

    pub fn line_complete(&self, index: usize, column: bool) -> bool {
        (0..SIDE).all(|offset| {
            self.is_solved(if column {
                offset * SIDE + index
            } else {
                index * SIDE + offset
            })
        })
    }

    pub fn select(&mut self, cell: usize) -> bool {
        if self.outcome != Outcome::Playing || cell >= CELLS || self.is_solved(cell) {
            return false;
        }
        self.cell = cell;
        true
    }

    pub fn move_selection(&mut self, horizontal: i32, vertical: i32) -> bool {
        if self.outcome != Outcome::Playing
            || !matches!((horizontal, vertical), (-1 | 1, 0) | (0, -1 | 1))
        {
            return false;
        }
        let mut row = (self.cell / SIDE) as i32 + vertical;
        let mut col = (self.cell % SIDE) as i32 + horizontal;
        while (0..SIDE as i32).contains(&row) && (0..SIDE as i32).contains(&col) {
            if self.select(row as usize * SIDE + col as usize) {
                return true;
            }
            row += vertical;
            col += horizontal;
        }
        false
    }

    pub fn take_reward(&mut self) -> u32 {
        if self.outcome != Outcome::Win || self.reward_claimed {
            return 0;
        }
        self.reward_claimed = true;
        POINTS_PER_WIN
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
        if self.outcome != Outcome::Playing || self.is_solved(self.cell) {
            return None;
        }
        let (row, column) = self.question();
        let correct = answer == self.operation.answer(row, column);
        self.last_lines = 0;
        if correct {
            let cell = self.cell;
            let direction = self.last_correct.and_then(|previous| {
                let horizontal = (cell % SIDE) as i32 - (previous % SIDE) as i32;
                let vertical = (cell / SIDE) as i32 - (previous / SIDE) as i32;
                (horizontal.abs() + vertical.abs() == 1).then_some((horizontal, vertical))
            });
            self.combo = if direction.is_some() {
                self.combo + 1
            } else {
                1
            };
            self.last_correct = Some(cell);
            self.solved |= 1u128 << cell;
            self.solved_count += 1;
            self.last_lines = u32::from(self.line_complete(cell / SIDE, false))
                + u32::from(self.line_complete(cell % SIDE, true));
            let damage = 2 + self.combo.min(2) + 12 * self.last_lines;
            self.opponent_hp = self.opponent_hp.saturating_sub(damage);
            if self.opponent_hp == 0 || self.solved_count as usize == CELLS {
                self.outcome = Outcome::Win;
            } else {
                let (horizontal, vertical) = direction.unwrap_or((1, 0));
                if !self.move_selection(horizontal, vertical)
                    && !self.move_selection(-horizontal, -vertical)
                {
                    if let Some(next) = (1..CELLS)
                        .map(|offset| (cell + offset) % CELLS)
                        .find(|&index| !self.is_solved(index))
                    {
                        self.select(next);
                    }
                }
            }
        } else {
            self.combo = 0;
            self.last_correct = None;
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
    pub extern "C" fn battle_health(opponent: u32) -> u32 {
        Battle::opponent_health(opponent as usize)
    }

    #[no_mangle]
    pub extern "C" fn battle_select(cell: u32) -> u32 {
        BATTLE.with_borrow_mut(|slot| {
            u32::from(
                slot.as_mut()
                    .is_some_and(|battle| battle.select(cell as usize)),
            )
        })
    }

    #[no_mangle]
    pub extern "C" fn battle_move(horizontal: i32, vertical: i32) -> u32 {
        BATTLE.with_borrow_mut(|slot| {
            u32::from(
                slot.as_mut()
                    .is_some_and(|battle| battle.move_selection(horizontal, vertical)),
            )
        })
    }

    #[no_mangle]
    pub extern "C" fn battle_reward() -> u32 {
        BATTLE.with_borrow_mut(|slot| slot.as_mut().map_or(0, Battle::take_reward))
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
                8 => b.solved_count,
                9 => b.last_lines,
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
        assert_eq!((battle.combo, battle.cell, battle.opponent_hp), (2, 2, 39));
        assert_eq!(battle.submit(answer(&battle) + 1, 30), Some(false));
        assert_eq!((battle.combo, battle.cell), (0, 2));
        for _ in 0..10 {
            battle.submit(answer(&battle), 40);
        }
        assert_eq!(battle.outcome, Outcome::Win);
        assert_eq!(battle.submit(0, 1_000_000), None);
        assert_eq!(battle.player_hp, 36);
        assert_eq!(battle.take_reward(), POINTS_PER_WIN);
        assert_eq!(battle.take_reward(), 0);
    }

    #[test]
    fn grid_selection_controls_chain_and_completes_a_vertical_line() {
        let mut battle = Battle::new(42, Operation::Add, 0);
        for row in 0..SIDE {
            assert!(battle.select(row * SIDE + 3));
            let (a, b) = battle.question();
            assert_eq!(battle.submit(a + b, 10), Some(true));
            assert!(battle.is_solved(row * SIDE + 3));
            assert!(!battle.select(row * SIDE + 3));
            if (1..SIDE - 1).contains(&row) {
                assert_eq!(battle.cell, (row + 1) * SIDE + 3);
            }
        }
        assert!(battle.line_complete(3, true));
        assert_eq!(
            (battle.solved_count, battle.combo, battle.last_lines),
            (10, 10, 1)
        );
        assert_eq!(battle.outcome, Outcome::Win);
        let mut battle = Battle::new(42, Operation::Multiply, 1);
        let (a, b) = battle.question();
        battle.submit(a * b, 10);
        assert!(battle.select(22));
        let (a, b) = battle.question();
        battle.submit(a * b, 10);
        assert_eq!(battle.combo, 1);
    }

    #[test]
    fn cpu_attacks_on_schedule_and_can_win() {
        let mut battle = Battle::new(42, Operation::Add, 11);
        battle.tick(5_699);
        assert_eq!(battle.player_hp, 36);
        battle.tick(11_400);
        assert_eq!((battle.player_hp, battle.attacks), (24, 2));
        assert_eq!(battle.submit(0, 35_000), None);
        assert_eq!(battle.outcome, Outcome::Loss);
        assert_eq!(battle.take_reward(), 0);
    }
}
