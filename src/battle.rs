use crate::collection::POINTS_PER_WIN;
use crate::{axes, Digits, Operation, CELLS, SIDE};

pub const OPPONENTS: [&str; 12] = [
    "ミジンコ",
    "ありんこ",
    "ネズミ",
    "うさぎ",
    "あかちゃん",
    "おんなのこ",
    "なだこうせい",
    "とうだいせい",
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
    pub cell: usize,
    pub last_lines: u32,
    pub opponent_solved: u32,
    pub solved_count: u32,
    solved: u128,
    last_correct: Option<usize>,
    pub outcome: Outcome,
    interval: u32,
    reward_claimed: bool,
}

impl Battle {
    pub fn new(seed: u32, operation: Operation, opponent: usize, digits: Digits) -> Self {
        let level = opponent.min(OPPONENTS.len() - 1) as u32;
        let base_interval = match level {
            0 => 7_500,
            1 => 6_500,
            _ => 5_000 - level * 330,
        };
        let interval = base_interval * digits.race_factor(operation);
        Self {
            numbers: axes(seed, digits),
            operation,
            cell: 0,
            last_lines: 0,
            opponent_solved: 0,
            solved_count: 0,
            solved: 0,
            last_correct: None,
            outcome: Outcome::Playing,
            interval,
            reward_claimed: false,
        }
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

    pub fn tick(&mut self, elapsed_ms: u32) {
        if self.outcome == Outcome::Playing {
            self.opponent_solved = (elapsed_ms / self.interval).min(CELLS as u32);
            if self.opponent_solved == CELLS as u32 {
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
            self.last_correct = Some(cell);
            self.solved |= 1u128 << cell;
            self.solved_count += 1;
            self.last_lines = u32::from(self.line_complete(cell / SIDE, false))
                + u32::from(self.line_complete(cell % SIDE, true));
            if self.solved_count as usize == CELLS {
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
    pub extern "C" fn battle_start(seed: u32, operation: u32, opponent: u32, digits: u32) {
        BATTLE.with_borrow_mut(|slot| {
            *slot = Some(Battle::new(
                seed,
                Operation::from_index(operation),
                opponent as usize,
                Digits::from_count(digits),
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
    pub extern "C" fn battle_value(field: u32) -> u32 {
        BATTLE.with_borrow(|slot| {
            slot.as_ref().map_or(0, |b| match field {
                0 => b.solved_count,
                1 => b.opponent_solved,
                2 => b.cell as u32,
                3 => b.outcome as u32,
                4 => b.last_lines,
                _ => 0,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(battle: &mut Battle, elapsed: u32) {
        let (row, col) = battle.question();
        assert_eq!(
            battle.submit(battle.operation.answer(row, col), elapsed),
            Some(true)
        );
    }

    #[test]
    fn win_and_reward_require_all_hundred_answers() {
        let mut race = Battle::new(42, Operation::Subtract, 0, Digits::One);
        for solved in 1..CELLS {
            answer(&mut race, solved as u32 * 2_000);
            assert_eq!(race.outcome, Outcome::Playing);
            assert_eq!(race.solved_count, solved as u32);
        }
        assert_eq!(race.take_reward(), 0);
        answer(&mut race, 200_000);
        assert_eq!(race.solved_count, 100);
        assert_eq!(race.opponent_solved, 26);
        assert_eq!(race.outcome, Outcome::Win);
        assert_eq!(race.take_reward(), POINTS_PER_WIN);
        assert_eq!(race.take_reward(), 0);
        race.tick(1_000_000);
        assert_eq!(race.opponent_solved, 26);
    }

    #[test]
    fn wrong_answer_does_not_fill_square_and_completed_line_does_not_end_race() {
        let mut race = Battle::new(42, Operation::Add, 0, Digits::One);
        assert!(race.select(3));
        assert_eq!(race.submit(99, 0), Some(false));
        assert_eq!(race.solved_count, 0);
        assert_eq!(race.cell, 3);
        for row in 0..SIDE {
            assert!(race.select(row * SIDE + 3));
            answer(&mut race, 0);
            assert!(race.is_solved(row * SIDE + 3));
            assert!(!race.select(row * SIDE + 3));
            if (1..SIDE - 1).contains(&row) {
                assert_eq!(race.cell, (row + 1) * SIDE + 3);
            }
        }
        assert!(race.line_complete(3, true));
        assert_eq!(race.last_lines, 1);
        assert_eq!(race.outcome, Outcome::Playing);
    }

    #[test]
    fn cpu_pace_accounts_for_three_digit_multiplication() {
        let mut race = Battle::new(42, Operation::Multiply, 0, Digits::Three);
        race.tick(5_000);
        assert_eq!(race.opponent_solved, 0);
        answer(&mut race, 5_000);
        race.tick(45_000);
        assert_eq!(race.opponent_solved, 1);
        assert_eq!(race.solved_count, 1);
    }

    #[test]
    fn beginner_opponents_take_longer_to_finish() {
        for (opponent, finish) in [(0, 750_000), (1, 650_000)] {
            let mut race = Battle::new(42, Operation::Add, opponent, Digits::UpToFive);
            race.tick(finish - 1);
            assert_eq!(race.opponent_solved, 99);
            assert_eq!(race.outcome, Outcome::Playing);
            race.tick(finish);
            assert_eq!(race.outcome, Outcome::Loss);
        }
    }

    #[test]
    fn cpu_finishes_only_after_solving_all_hundred_cells() {
        let mut race = Battle::new(42, Operation::Add, 11, Digits::One);
        race.tick(136_999);
        assert_eq!(race.opponent_solved, 99);
        assert_eq!(race.outcome, Outcome::Playing);
        assert_eq!(race.submit(0, 137_000), None);
        assert_eq!(race.opponent_solved, 100);
        assert_eq!(race.outcome, Outcome::Loss);
        assert_eq!(race.take_reward(), 0);
    }
}
