use std::{collections::HashMap, fmt::Write as _};

use indoc::{formatdoc, indoc};
use serenity::all::{CreateEmbed, CreateEmbedFooter, GuildId, Mention, UserId};
use sqlx::{FromRow, PgPool};
use tracing::{debug, error, info};

use crate::game::{CalculateAllTimeError, CalculateDailyError};

#[derive(Clone, Debug)]
pub struct Daily {
    day: usize,
    pub entries: Vec<DailyEntry>,
}

impl Daily {
    pub async fn calculate_for(
        db_pool: &PgPool,
        guild_id: GuildId,
        day: usize,
    ) -> Result<Self, CalculateDailyError> {
        let get_scores = sqlx::query(indoc! {"
            SELECT
                s.user_id,
                s.score
            FROM
                flagle_scores s
                INNER JOIN users u USING (user_id)
            WHERE
                s.guild_id = $1
                AND s.board = $2
                AND s.board = s.day_added
                AND s.score != 0
            ORDER BY score DESC;
        "});
        let entries = match get_scores
            .bind(guild_id.get() as i64)
            .bind(day as i32)
            .fetch_all(db_pool)
            .await
        {
            Ok(rows) => {
                info!("fetched all scores");

                rows.into_iter()
                    .map(|row| {
                        DailyQueryRow::from_row(&row)
                            .map(|row| {
                                #[cfg(debug_assertions)]
                                debug!(?row, "got leaderboard entry");
                                row.into()
                            })
                            .map_err(CalculateDailyError::FromRow)
                    })
                    .collect::<Result<Vec<_>, CalculateDailyError>>()?
            }
            Err(error) => {
                error!(%error, "failed to fetch all scores");
                return Err(CalculateDailyError::Unexpected(error));
            }
        };

        Ok(Daily { day, entries })
    }
}

impl From<Daily> for CreateEmbed {
    fn from(leaderboard: Daily) -> Self {
        let mut embed = CreateEmbed::new()
            .title("Today's Flagle Leaderboard")
            .field("board", format!("{}", leaderboard.day), true);

        let mut description = String::new();

        let mut last_score = usize::MAX;
        let mut duplicates = 0;

        for (i, entry) in leaderboard.entries.into_iter().enumerate() {
            if last_score == entry.score {
                duplicates += 1;
            } else {
                duplicates = 0;
            };

            writeln!(
                &mut description,
                "- {}. {} ({} pts)",
                i + 1 - duplicates,
                Mention::User(entry.user_id),
                entry.score,
            )
            .expect("should be able to write into String");

            last_score = entry.score;
        }

        embed = embed
            .description(description)
            .footer(CreateEmbedFooter::new(
                "Ranking may change with more submissions! Run `/leaderboard` again to see \
                 updated scores.",
            ));

        embed
    }
}

#[derive(Clone, Debug)]
pub struct DailyEntry {
    pub user_id: UserId,
    pub score: usize,
}

impl From<DailyQueryRow> for DailyEntry {
    fn from(row: DailyQueryRow) -> Self {
        Self {
            user_id: UserId::new(row.user_id as u64),
            score: row.score as usize,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
struct DailyQueryRow {
    user_id: i64,
    score: i32,
}

#[derive(Clone, Debug)]
pub struct AllTime {
    end_day: usize,
    include_end: bool,
    include_late: bool,
    pub scores_listing: Vec<(UserId, usize)>,
}

impl AllTime {
    pub async fn calculate(
        db_pool: &PgPool,
        guild_id: GuildId,
        end_day: usize,
        include_end: bool,
        include_late: bool,
    ) -> Result<Self, CalculateAllTimeError> {
        let board_clause = if include_end {
            "AND s.board <= $2"
        } else {
            "AND s.board < $2"
        };

        let late_clause = if include_late {
            ""
        } else {
            "AND s.board = s.day_added"
        };

        let get_scores_string = formatdoc!(
            "
            SELECT
                s.user_id,
                s.score
            FROM
                flagle_scores s
                INNER JOIN users u USING (user_id)
            WHERE
                s.guild_id = $1
                AND s.score != 0
                {}
                {};
            ",
            board_clause,
            late_clause
        );
        let get_scores = sqlx::query(get_scores_string.as_ref());
        let scores = match get_scores
            .bind(guild_id.get() as i64)
            .bind(end_day as i32)
            .fetch_all(db_pool)
            .await
        {
            Ok(rows) => {
                info!(num = %rows.len(), "fetched all scores");

                let rows = rows
                    .into_iter()
                    .map(|row| {
                        AllTimeQueryRow::from_row(&row).map_err(CalculateAllTimeError::FromRow)
                    })
                    .collect::<Result<Vec<_>, CalculateAllTimeError>>()?;

                let mut scores = HashMap::<UserId, usize>::default();

                for row in rows {
                    debug!(?row, "got row");

                    let id = UserId::new(row.user_id as u64);
                    let score = scores.entry(id).or_insert(0);

                    *score += row.score as usize;
                }

                scores
            }
            Err(error) => {
                error!(%error, "failed to fetch all scores");
                return Err(CalculateAllTimeError::Unexpected(error));
            }
        };

        info!(?scores, "scores table");

        let mut scores_listing: Vec<_> = scores.into_iter().collect();
        scores_listing.sort_by_key(|sl| sl.1);
        scores_listing.reverse();

        info!(?scores_listing, "scores listing");

        Ok(AllTime {
            end_day,
            include_end,
            include_late,
            scores_listing,
        })
    }
}

impl From<AllTime> for CreateEmbed {
    fn from(leaderboard: AllTime) -> Self {
        let mut embed = CreateEmbed::new()
            .title("All-Time Flagle Leaderboard")
            .field(
                format!("Includes today's board (#{})?", leaderboard.end_day),
                if leaderboard.include_end { "Yes" } else { "No" },
                true,
            )
            .field(
                "Includes late submissions?",
                if leaderboard.include_late {
                    "Yes"
                } else {
                    "No"
                },
                true,
            );

        let mut description = String::new();

        let mut last_score = usize::MAX;
        let mut duplicates = 0;

        for (i, (user_id, score)) in leaderboard.scores_listing.into_iter().enumerate() {
            if last_score == score {
                duplicates += 1;
            } else {
                duplicates = 0;
            };

            writeln!(
                &mut description,
                "- {}. {}: {}",
                i + 1 - duplicates,
                Mention::User(user_id),
                score,
            )
            .expect("should be able to write into String");

            last_score = score;
        }

        embed = embed
            .description(description)
            .footer(CreateEmbedFooter::new(
                "Ranking may change with more submissions! Run `/leaderboard` again to see \
                 updated scores.",
            ));

        embed
    }
}

#[derive(Clone, Debug, FromRow)]
struct AllTimeQueryRow {
    user_id: i64,
    score: i32,
}
