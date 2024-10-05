use chrono::{DateTime, Days, NaiveDate, Utc};
use tap::TryConv;

/// Get the date on which a board with a given number occurred.
///
/// Panics if `number == 0`.
pub fn date_of_board(number: usize) -> NaiveDate {
    start_date() + Days::new(number as u64 - 1)
}

/// Get the date on which board 1 occurred.
fn start_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2022, 2, 22).expect("22 February 2022 is a valid date")
}

/// Get today's date, in UTC since it's not clear which timezone Flagle uses.
pub fn today() -> NaiveDate {
    date_from_utc(Utc::now())
}

pub fn date_from_utc(date: DateTime<Utc>) -> NaiveDate {
    // We have to assume Flagle dates are in UTC, since flagle.io doesn't say
    date.date_naive()
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
    fn board_1_is_22_feb_2022() {
        let manual_date =
            NaiveDate::from_ymd_opt(2022, 2, 22).expect("22 February 2022 is a valid date");
        let calc_date = super::date_of_board(1);

        assert_eq!(manual_date, calc_date);
    }

    #[test]
    fn board_on_22_feb_2022_is_1() {
        let manual_date =
            NaiveDate::from_ymd_opt(2022, 2, 22).expect("22 February 2022 is a valid date");
        let board_num = super::board_on_date(manual_date);

        assert_eq!(board_num, Some(1));
    }

    #[test]
    fn board_957_is_5_oct_2024() {
        let manual_date = NaiveDate::from_ymd_opt(2024, 10, 5).expect("5 Oct 2024 is a valid date");
        let calc_date = super::date_of_board(957);

        assert_eq!(manual_date, calc_date);
    }

    #[test]
    fn board_on_5_oct_2024_is_957() {
        let manual_date = NaiveDate::from_ymd_opt(2024, 10, 5).expect("5 Oct 2024 is a valid date");
        let board_num = super::board_on_date(manual_date);

        assert_eq!(board_num, Some(957));
    }
}
