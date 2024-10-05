use std::fmt;

use indoc::indoc;
use serenity::all::{GuildId, User};
use sqlx::{Error as SqlxError, FromRow, Postgres, Transaction};
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Clone, Debug, FromRow)]
pub struct UserRow {
    pub user_id: i64,
}

#[derive(Clone, Debug, FromRow)]
pub struct GuildRow {
    pub guild_id: i64,
}

#[derive(Clone, Debug, FromRow)]
pub struct GuildUserRow {
    pub guild_id: i64,
    pub user_id: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InsertionTarget {
    Guild,
    User,
    GuildUser,
    Score,
}

impl fmt::Display for InsertionTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InsertionTarget::Guild => write!(f, "guild"),
            InsertionTarget::User => write!(f, "user"),
            InsertionTarget::GuildUser => write!(f, "guild user"),
            InsertionTarget::Score => write!(f, "score"),
        }
    }
}

pub async fn insert_guild_user(
    txn: &mut Transaction<'_, Postgres>,
    guild_id: GuildId,
    user: &User,
) -> Result<(), GuildUserInsertionError> {
    let insert_guilds = sqlx::query(indoc! {"
        INSERT INTO guilds (guild_id)
        VALUES ($1)
        ON CONFLICT (guild_id) DO NOTHING;
    "});
    match insert_guilds
        .bind(guild_id.get() as i64)
        .execute(txn.as_mut())
        .await
    {
        Ok(result) if result.rows_affected() > 0 => info!(
            guild_id = %guild_id.get() as i64,
            "inserted new guild"
        ),
        Ok(_) => debug!(
            guild_id = %guild_id.get() as i64,
            "guild already exists in guilds table"
        ),
        Err(error) => {
            error!(%error, "failed to insert guild");
            return Err(GuildUserInsertionError::UnexpectedSqlx {
                target: InsertionTarget::Guild,
                error,
            });
        }
    }

    let insert_users = sqlx::query(indoc! {"
        INSERT INTO users (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO NOTHING;
    "});
    match insert_users
        .bind(user.id.get() as i64)
        .execute(txn.as_mut())
        .await
    {
        Ok(_) => info!("inserted new user or updated existing"),
        Err(error) => {
            error!(%error, "failed to insert user");
            return Err(GuildUserInsertionError::UnexpectedSqlx {
                target: InsertionTarget::User,
                error,
            });
        }
    }

    let insert_guild_users = sqlx::query(indoc! {"
        INSERT INTO guild_users (guild_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING;
    "});
    match insert_guild_users
        .bind(guild_id.get() as i64)
        .bind(user.id.get() as i64)
        .execute(txn.as_mut())
        .await
    {
        Ok(result) if result.rows_affected() > 0 => info!("inserted new guild user"),
        Ok(_) => debug!("guild user already exists in guild_users table"),
        Err(error) => {
            error!(%error, "failed to insert guild user");
            return Err(GuildUserInsertionError::UnexpectedSqlx {
                target: InsertionTarget::GuildUser,
                error,
            });
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum GuildUserInsertionError {
    #[error("unexpected SQLx error when inserting {target}: {error}")]
    UnexpectedSqlx {
        target: InsertionTarget,
        #[source]
        error: SqlxError,
    },
}
