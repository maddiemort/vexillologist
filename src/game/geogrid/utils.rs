use chrono::{DateTime, Days, FixedOffset, NaiveDate, Utc};
use tap::TryConv;

/// Get the date on which a board with a given number occurred.
///
/// Panics if `number == 0`.
pub fn date_of_board(number: usize) -> NaiveDate {
    start_date() + Days::new(number as u64 - 1)
}

/// Get the date on which board 1 occurred.
fn start_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 4, 7).expect("7 April 2024 is a valid date")
}

/// Get today's date, in the appropriate timezone for Geogrid.
pub fn today() -> NaiveDate {
    date_from_utc(Utc::now())
}

pub fn date_from_utc(date: DateTime<Utc>) -> NaiveDate {
    // Geogrid dates are all in EST, according to the info on geogridgame.com
    let est = FixedOffset::west_opt(4 * 3600).expect("4 hours should be a valid timezone offset");
    date.with_timezone(&est).date_naive()
}

/// Get the number of the board that occurred on `date`. Returns `None` if `date` was before day 1.
pub fn board_on_date(date: NaiveDate) -> Option<usize> {
    let days_since = (date - start_date()).num_days();
    days_since
        .try_conv::<usize>()
        .ok()
        .map(|pos_days_since| pos_days_since + 1)
}

/// Get the number of the board that is active right now.
pub fn board_now() -> usize {
    board_on_date(today()).expect("today is always after day 1")
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    #[test]
    fn board_1_is_7_apr_2024() {
        let manual_date =
            NaiveDate::from_ymd_opt(2024, 4, 7).expect("7 April 2024 is a valid date");
        let calc_date = super::date_of_board(1);

        assert_eq!(manual_date, calc_date);
    }

    #[test]
    fn board_on_7_apr_2024_is_1() {
        let manual_date =
            NaiveDate::from_ymd_opt(2024, 4, 7).expect("7 April 2024 is a valid date");
        let board_num = super::board_on_date(manual_date);

        assert_eq!(board_num, Some(1));
    }

    #[test]
    fn board_79_is_24_jun_2024() {
        let manual_date =
            NaiveDate::from_ymd_opt(2024, 6, 24).expect("24 June 2024 is a valid date");
        let calc_date = super::date_of_board(79);

        assert_eq!(manual_date, calc_date);
    }

    #[test]
    fn board_on_24_jun_2024_is_79() {
        let manual_date =
            NaiveDate::from_ymd_opt(2024, 6, 24).expect("24 June 2024 is a valid date");
        let board_num = super::board_on_date(manual_date);

        assert_eq!(board_num, Some(79));
    }
}
