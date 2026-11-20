pub const POINTS_PER_WIN: u32 = 10;
pub const POINTS_PER_COIN: u32 = 10;
pub const COIN_COUNT: usize = 200;

pub fn unlocked_coins(points: u32) -> usize {
    ((points / POINTS_PER_COIN) as usize).min(COIN_COUNT)
}

#[cfg(windows)]
pub const CATALOG: &str = include_str!("../web/coins/catalog.tsv");

#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn coin_count(points: u32) -> u32 {
    unlocked_coins(points) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_entries_have_a_name_dates_and_summary() {
        let catalog = include_str!("../web/coins/catalog.tsv");
        let entries: Vec<_> = catalog
            .lines()
            .filter(|line| !line.starts_with("# "))
            .collect();
        assert_eq!(entries.len(), COIN_COUNT);
        for line in entries {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 3);
            assert!(fields.iter().all(|field| !field.is_empty()));
        }
    }

    #[test]
    fn each_win_unlocks_one_coin_until_collection_is_full() {
        assert_eq!(unlocked_coins(0), 0);
        assert_eq!(unlocked_coins(POINTS_PER_WIN), 1);
        assert_eq!(unlocked_coins(POINTS_PER_WIN * 2), 2);
        assert_eq!(unlocked_coins(u32::MAX), COIN_COUNT);
    }
}
