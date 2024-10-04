use std::{fmt, str::FromStr};

use serenity::model::prelude::{GuildId, User};
use sqlx::PgPool;
use thiserror::Error;

use crate::game::ScoreInsertionError;

mod persist;
pub mod utils;

pub struct Geogrid;

impl super::Game for Geogrid {
    type Score = Score;

    fn description() -> &'static str {
        "Geogrid"
    }
}

impl super::Score for Score {
    type Game = Geogrid;

    async fn insert(
        self,
        db_pool: &PgPool,
        guild_id: GuildId,
        user: &User,
    ) -> Result<impl super::InsertedScore<Game = Geogrid>, ScoreInsertionError> {
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

        Ok(Score {
            correct,
            board,
            score,
            rank,
            players,
        })
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
    fn parse_all_correct() {
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
    fn parse_mixed() {
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
}
