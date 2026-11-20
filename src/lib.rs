pub const SIDE: usize = 10;
pub const CELLS: usize = SIDE * SIDE;

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

pub fn axes(seed: u32) -> [i32; SIDE * 2] {
    let mut state = seed.max(1);
    let mut numbers = [0; SIDE * 2];
    for axis in numbers.chunks_exact_mut(SIDE) {
        for (index, number) in axis.iter_mut().enumerate() {
            *number = index as i32;
        }
        for index in (1..SIDE).rev() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            axis.swap(index, state as usize % (index + 1));
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
    pub extern "C" fn make_axis(seed: u32, axis: u32) -> u64 {
        let board = axes(seed);
        let start = (axis.min(1) as usize) * SIDE;
        board[start..start + SIDE]
            .iter()
            .fold(0, |packed, &digit| packed * 10 + digit as u64)
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
            let board = axes(seed);
            assert_eq!(board, axes(seed));
            for axis in board.chunks_exact(SIDE) {
                let mut sorted = axis.to_vec();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..10).collect::<Vec<_>>());
            }
        }
        assert_ne!(axes(1), axes(42));
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
