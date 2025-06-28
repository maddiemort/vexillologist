use std::{fmt, str::FromStr};

use indoc::indoc;
use serenity::{
    all::CreateEmbed,
    model::prelude::{GuildId, User},
};
use sqlx::{Error as SqlxError, PgPool};
use thiserror::Error;
use tracing::{error, info, instrument};

use crate::game::{
    geogrid::leaderboards::{AllTime, Board, Daily},
    CalculateAllTimeError, CalculateBoardError, CalculateDailyError, ScoreInsertionError,
    UserScoreCheckError,
};

pub mod leaderboards;
pub mod persist;
pub mod utils;

pub struct GeoGrid;

impl super::Game for GeoGrid {
    type Score = Score;

    const NAME: &'static str = "GeoGrid";
    const LINK: &'static str = "https://www.geogridgame.com";

    async fn daily_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateDailyError> {
        Daily::calculate_for(db_pool, guild_id, utils::board_now()).await
    }

    async fn all_time_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
        include_today: bool,
        include_late: bool,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateAllTimeError> {
        AllTime::calculate(
            db_pool,
            guild_id,
            utils::board_now(),
            include_today,
            include_late,
        )
        .await
    }

    async fn board_leaderboard(
        db_pool: &PgPool,
        guild_id: GuildId,
        board: usize,
    ) -> Result<impl Into<CreateEmbed> + fmt::Debug, CalculateBoardError> {
        Board::calculate_for(db_pool, guild_id, board).await
    }

    #[instrument(skip_all, fields(game = %Self::NAME, %guild_id, user_id = %user.id))]
    async fn user_ever_scored(
        db_pool: &sqlx::PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<bool, UserScoreCheckError> {
        info!("checking if user has ever recorded a score for this game");

        let get_any_score = sqlx::query(indoc! {"
            SELECT user_id FROM geogrid_scores
            WHERE
                guild_id = $1
                AND user_id = $2
            LIMIT 1;
        "});

        match get_any_score
            .bind(guild_id.get() as i64)
            .bind(user.id.get() as i64)
            .fetch_one(db_pool)
            .await
        {
            Ok(_) => {
                info!("user has previously submitted at least one score for this game");
                Ok(true)
            }
            Err(SqlxError::RowNotFound) => {
                info!("user has never submitted a score for this game");
                Ok(false)
            }
            Err(error) => {
                error!(%error, "failed to check for user's scores");
                Err(UserScoreCheckError::Unexpected(error))
            }
        }
    }

    #[instrument(skip_all, fields(game = %Self::NAME, %guild_id, user_id = %user.id))]
    async fn user_scored_today(
        db_pool: &sqlx::PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<bool, UserScoreCheckError> {
        let board_now = utils::board_now();
        info!(%board_now, "checking if user has recorded a score for this game today");

        let get_todays_score = sqlx::query(indoc! {"
            SELECT user_id FROM geogrid_scores
            WHERE
                guild_id = $1
                AND user_id = $2
                AND board = $3
            LIMIT 1;
        "});

        match get_todays_score
            .bind(guild_id.get() as i64)
            .bind(user.id.get() as i64)
            .bind(board_now as i32)
            .fetch_one(db_pool)
            .await
        {
            Ok(_) => {
                info!("user has submitted a score for this game today");
                Ok(true)
            }
            Err(SqlxError::RowNotFound) => {
                info!("user has not submitted a score for this game today");
                Ok(false)
            }
            Err(error) => {
                error!(%error, "failed to check for user's scores");
                Err(UserScoreCheckError::Unexpected(error))
            }
        }
    }
}

impl super::Score for Score {
    type Game = GeoGrid;

    async fn insert(
        self,
        db_pool: &PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<impl super::InsertedScore, ScoreInsertionError> {
        persist::insert_score(db_pool, self, guild_id, user).await
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub correct: usize,
    pub board: usize,
    pub score: f32,
    pub rank: usize,
    pub players: usize,
}

impl FromStr for Score {
    type Err = ParseScoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<ScoreV0>()
            .map(|v0| v0.0)
            .or_else(|_| s.parse::<ScoreV1>().map(|v1| v1.0))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ScoreV0(Score);

impl FromStr for ScoreV0 {
    type Err = ParseScoreError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let mut lines = raw.trim().lines();

        let (first, second, third) = (
            lines.next().ok_or(ParseScoreError::Empty)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
        );

        let grid_raw = first.trim().to_owned() + second.trim() + third.trim();
        let grid = grid_raw
            .chars()
            .filter_map(|c| match c {
                '✅' => Some(true),
                '❌' => Some(false),
                _ => None,
            })
            .collect::<Vec<_>>();

        if grid.is_empty() {
            return Err(ParseScoreError::Missing(Section::Grid));
        } else if grid.len() != 9 {
            return Err(ParseScoreError::InvalidFormat(Section::Grid));
        }

        let correct = grid.into_iter().filter(|&v| v).count();

        if lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .chars()
            .any(|c| !c.is_whitespace())
        {
            return Err(ParseScoreError::Missing(Section::Separator));
        }

        if lines.next() != Some("🌎Game Summary🌎") {
            return Err(ParseScoreError::Missing(Section::SummaryTitle));
        }

        let board = lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .strip_prefix("Board #")
            .ok_or(ParseScoreError::Missing(Section::BoardNumber))?
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Board))?;

        let score = lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .strip_prefix("Score: ")
            .ok_or(ParseScoreError::Missing(Section::Score))?
            .parse::<f32>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Score))?;

        let ranking_line = lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .strip_prefix("Rank: ")
            .ok_or(ParseScoreError::Missing(Section::Ranking))?;

        let (rank_raw, players_raw) = ranking_line
            .split_once(" / ")
            .ok_or(ParseScoreError::InvalidFormat(Section::Ranking))?;

        let rank = String::from_iter(rank_raw.chars().filter(|&c| c != ','))
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Rank))?;

        let players = String::from_iter(players_raw.chars().filter(|&c| c != ','))
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Players))?;

        Ok(ScoreV0(Score {
            correct,
            board,
            score,
            rank,
            players,
        }))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ScoreV1(Score);

impl FromStr for ScoreV1 {
    type Err = ParseScoreError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let mut lines = raw.trim().lines();

        let (first, second, third) = (
            lines.next().ok_or(ParseScoreError::Empty)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
            lines.next().ok_or(ParseScoreError::Truncated)?,
        );

        let grid_raw = first.trim().to_owned() + second.trim() + third.trim();
        let grid = grid_raw
            .chars()
            .filter_map(|c| match c {
                '🟩' => Some(true),
                '❌' => Some(false),
                _ => None,
            })
            .collect::<Vec<_>>();

        if grid.is_empty() {
            return Err(ParseScoreError::Missing(Section::Grid));
        } else if grid.len() != 9 {
            return Err(ParseScoreError::InvalidFormat(Section::Grid));
        }

        let correct = grid.into_iter().filter(|&v| v).count();

        let (score_raw, ranking_raw) = lines
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .split_once('|')
            .ok_or(ParseScoreError::InvalidFormat(Section::ScoreRanking))?;

        let score = score_raw
            .strip_prefix("Score:")
            .ok_or(ParseScoreError::InvalidFormat(Section::Score))?
            .trim()
            .parse::<f32>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Score))?;

        let (rank_raw, players_raw) = ranking_raw
            .trim()
            .strip_prefix("Rank:")
            .ok_or(ParseScoreError::InvalidFormat(Section::Ranking))?
            .trim()
            .split_once('/')
            .ok_or(ParseScoreError::InvalidFormat(Section::Ranking))?;

        let rank = String::from_iter(rank_raw.trim().chars().filter(|&c| c != ','))
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Rank))?;

        let players = String::from_iter(players_raw.trim().chars().filter(|&c| c != ','))
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Players))?;

        let mut find_board = lines.skip_while(|line| !line.starts_with("Board #"));

        let (board_raw, _) = find_board
            .next()
            .ok_or(ParseScoreError::Truncated)?
            .split_once('|')
            .ok_or(ParseScoreError::InvalidFormat(Section::BoardNumber))?;

        let board = board_raw
            .strip_prefix("Board #")
            .ok_or(ParseScoreError::Missing(Section::BoardNumber))?
            .trim()
            .parse::<usize>()
            .map_err(|_| ParseScoreError::NotANumber(Number::Board))?;

        Ok(ScoreV1(Score {
            correct,
            board,
            score,
            rank,
            players,
        }))
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseScoreError {
    #[error("string is empty")]
    Empty,

    #[error("string ends prematurely")]
    Truncated,

    #[error("string does not contain a {0}")]
    Missing(Section),

    #[error("{0} was not formatted as expected")]
    InvalidFormat(Section),

    #[error("{0} is not a number")]
    NotANumber(Number),
}

#[derive(Copy, Clone, Debug)]
pub enum Section {
    Grid,
    Separator,
    SummaryTitle,
    BoardNumber,
    Score,
    Ranking,
    ScoreRanking,
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Section::Grid => write!(f, "grid section"),
            Section::Separator => write!(f, "blank separator line"),
            Section::SummaryTitle => write!(f, "summary title"),
            Section::BoardNumber => write!(f, "board number line"),
            Section::Score => write!(f, "score line"),
            Section::Ranking => write!(f, "ranking line"),
            Section::ScoreRanking => write!(f, "score and ranking line"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Number {
    Board,
    Score,
    Rank,
    Players,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Board => write!(f, "board number"),
            Number::Score => write!(f, "score"),
            Number::Rank => write!(f, "rank"),
            Number::Players => write!(f, "player count"),
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::Score;

    #[test]
    fn parse_all_correct_v0() {
        let raw = indoc! {"
            ✅ ✅ ✅
            ✅ ✅ ✅
            ✅ ✅ ✅

            🌎Game Summary🌎
            Board #41
            Score: 114.7
            Rank: 2,213 / 9,015
            https://geogridgame.com/
            @geogridgame
        "};

        let score = raw
            .parse::<Score>()
            .expect("should have successfully parsed raw string to Score");

        assert_eq!(score.correct, 9);
        assert_eq!(score.board, 41);
        assert_eq!(score.score, 114.7);
        assert_eq!(score.rank, 2213);
        assert_eq!(score.players, 9015);
    }

    #[test]
    fn parse_mixed_v0() {
        let raw = indoc! {"
            ✅ ✅ ✅
            ✅ ✅ ✅
            ✅ ✅ ❌

            🌎Game Summary🌎
            Board #38
            Score: 193.7
            Rank: 2,387 / 7,102
            https://geogridgame.com
            @geogridgame
        "};

        let score = raw
            .parse::<Score>()
            .expect("should have successfully parsed raw string to Score");

        assert_eq!(score.correct, 8);
        assert_eq!(score.board, 38);
        assert_eq!(score.score, 193.7);
        assert_eq!(score.rank, 2387);
        assert_eq!(score.players, 7102);
    }

    #[test]
    fn parse_all_incorrect_v1() {
        let raw = indoc! {"
            ❌❌❌
            ❌❌❌
            ❌❌❌
            Score: 900 | Rank: 5,380/5,548
            Peak Performance 🚀 | ★★★★★
            My best square beat 100% of #geogridgame players!
            Board #439 | ♾️ Mode: Off
            https://geogridgame.com
        "};

        let score = raw
            .parse::<Score>()
            .expect("should have successfully parsed raw string to Score");

        assert_eq!(score.correct, 0);
        assert_eq!(score.board, 439);
        assert_eq!(score.score, 900.);
        assert_eq!(score.rank, 5380);
        assert_eq!(score.players, 5548);
    }

    #[test]
    fn parse_mixed_v1() {
        let raw = indoc! {"
            🟩🟩❌
            ❌🟩❌
            ❌🟩❌
            Score: 547.9 | Rank: 4,106/6,245
            Elite Among Mortals 🎖️
            Ordinary among #geogridgame savants, extraordinary among mere mortals.
            Board #440 | ♾️ Mode: Off
            https://geogridgame.com
        "};

        let score = raw
            .parse::<Score>()
            .expect("should have successfully parsed raw string to Score");

        assert_eq!(score.correct, 4);
        assert_eq!(score.board, 440);
        assert_eq!(score.score, 547.9);
        assert_eq!(score.rank, 4106);
        assert_eq!(score.players, 6245);
    }
}
