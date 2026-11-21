pub mod battle;
pub mod collection;

pub const SIDE: usize = 10;
pub const CELLS: usize = SIDE * SIDE;
pub const MAX_ANSWER_DIGITS: usize = 6;

#[derive(Clone, Copy, Debug)]
pub enum Digits {
    One,
    Two,
    Three,
}

impl Digits {
    pub const LABELS: [&'static str; 3] = ["1桁", "2桁", "3桁"];

    pub fn from_count(count: u32) -> Self {
        match count {
            2 => Self::Two,
            3 => Self::Three,
            _ => Self::One,
        }
    }

    fn bounds(self) -> (i32, i32) {
        match self {
            Self::One => (0, 10),
            Self::Two => (10, 100),
            Self::Three => (100, 1_000),
        }
    }

    fn race_factor(self, operation: Operation) -> u32 {
        match (self, operation) {
            (Self::One, _) => 1,
            (Self::Two, Operation::Multiply) => 3,
            (Self::Two, _) => 2,
            (Self::Three, Operation::Multiply) => 6,
            (Self::Three, _) => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
}

impl Operation {
    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Subtract,
            2 => Self::Multiply,
            _ => Self::Add,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
        }
    }

    pub fn answer(self, row: i32, column: i32) -> i32 {
        match self {
            Self::Add => row + column,
            Self::Subtract => row - column,
            Self::Multiply => row * column,
        }
    }
}

pub fn axes(seed: u32, digits: Digits) -> [i32; SIDE * 2] {
    let mut state = seed.max(1);
    let mut next = |range: i32| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state % range as u32) as i32
    };
    let (min, max) = digits.bounds();
    let mut numbers = [0; SIDE * 2];
    for axis in numbers.chunks_exact_mut(SIDE) {
        if matches!(digits, Digits::One) {
            for (index, number) in axis.iter_mut().enumerate() {
                *number = index as i32;
            }
            for index in (1..SIDE).rev() {
                axis.swap(index, next((index + 1) as i32) as usize);
            }
        } else {
            for index in 0..axis.len() {
                loop {
                    let number = min + next(max - min);
                    if !axis[..index].contains(&number) {
                        axis[index] = number;
                        break;
                    }
                }
            }
        }
    }
    numbers
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Mark {
    Blank,
    Incorrect,
    Correct,
}

pub fn mark(operation: Operation, row: i32, column: i32, answer: Option<i32>) -> Mark {
    match answer {
        None => Mark::Blank,
        Some(value) if value == operation.answer(row, column) => Mark::Correct,
        Some(_) => Mark::Incorrect,
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    #[no_mangle]
    pub extern "C" fn board_number(seed: u32, digits: u32, index: u32) -> i32 {
        axes(seed, Digits::from_count(digits))
            .get(index as usize)
            .copied()
            .unwrap_or(0)
    }

    #[no_mangle]
    pub extern "C" fn grade(
        operation: u32,
        row: i32,
        column: i32,
        has_answer: u32,
        answer: i32,
    ) -> u32 {
        mark(
            Operation::from_index(operation),
            row,
            column,
            (has_answer != 0).then_some(answer),
        ) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_axes_are_permutations_and_the_seed_is_reproducible() {
        for seed in [0, 1, 42, u32::MAX] {
            let board = axes(seed, Digits::One);
            assert_eq!(board, axes(seed, Digits::One));
            for axis in board.chunks_exact(SIDE) {
                let mut sorted = axis.to_vec();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..10).collect::<Vec<_>>());
            }
        }
        assert_ne!(axes(1, Digits::One), axes(42, Digits::One));
    }

    #[test]
    fn multi_digit_axes_use_distinct_numbers_of_the_selected_length() {
        for (digits, range) in [(Digits::Two, 10..100), (Digits::Three, 100..1000)] {
            for seed in [0, 42, u32::MAX] {
                let board = axes(seed, digits);
                assert_eq!(board, axes(seed, digits));
                for axis in board.chunks_exact(SIDE) {
                    assert!(axis.iter().all(|number| range.contains(number)));
                    assert_eq!(
                        axis.iter().collect::<std::collections::HashSet<_>>().len(),
                        SIDE
                    );
                }
            }
        }
        assert_eq!(Operation::Multiply.answer(999, 999), 998001);
        assert_eq!(Operation::Subtract.answer(100, 999), -899);
    }

    #[test]
    fn grades_all_operations_and_distinguishes_blank_from_zero() {
        assert_eq!(mark(Operation::Add, 2, 3, Some(5)), Mark::Correct);
        assert_eq!(mark(Operation::Subtract, 2, 3, Some(-1)), Mark::Correct);
        assert_eq!(mark(Operation::Multiply, 0, 3, Some(0)), Mark::Correct);
        assert_eq!(mark(Operation::Multiply, 0, 3, None), Mark::Blank);
        assert_eq!(mark(Operation::Subtract, 2, 3, Some(1)), Mark::Incorrect);
    }
}
