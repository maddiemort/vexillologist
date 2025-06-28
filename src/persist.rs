use std::fmt;

use indoc::indoc;
use serenity::all::{GuildId, User, UserId};
use sqlx::{Error as SqlxError, Executor, FromRow, Postgres, Row as _, Transaction};
use thiserror::Error;
use tracing::{debug, error, info, instrument};

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
    pub opted_out: bool,
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

#[instrument(skip_all, fields(user_id = %user.id, %guild_id))]
pub async fn is_opted_out(
    executor: impl Executor<Database = Postgres, '_>,
    guild_id: GuildId,
    user: &User,
) -> Result<bool, OptOutCheckError> {
    info!("checking for opt-out for user");

    let select_opted_out = sqlx::query(indoc! {"
        SELECT opted_out FROM guild_users
        WHERE
            guild_id = $1
            AND user_id = $2
        LIMIT 1;
    "});

    match select_opted_out
        .bind(guild_id.get() as i64)
        .bind(user.id.get() as i64)
        .fetch_one(executor)
        .await
        .and_then(|row| row.try_get::<bool, _>(0))
    {
        Ok(opted_out) => {
            debug!("found existing guild-user entry");
            Ok(opted_out)
        }
        Err(SqlxError::RowNotFound) => {
            debug!("no guild-user entry found");
            Ok(false)
        }
        Err(error) => {
            error!(%error, "failed to check opt-out status for user");
            Err(OptOutCheckError::UnexpectedSqlx {
                user_id: user.id,
                guild_id,
                error,
            })
        }
    }
}

#[derive(Debug, Error)]
pub enum OptOutCheckError {
    #[error(
        "unexpected SQLx error when checking opt-out for user {user_id} in guild {guild_id}: \
         {error}"
    )]
    UnexpectedSqlx {
        user_id: UserId,
        guild_id: GuildId,
        #[source]
        error: SqlxError,
    },
}

#[instrument(skip_all, fields(user_id = %user.id, %guild_id, %opted_out))]
pub async fn set_opt_out(
    executor: impl Executor<Database = Postgres, '_>,
    guild_id: GuildId,
    user: &User,
    opted_out: bool,
) -> Result<(), OptOutSetError> {
    info!("setting opt-out for user");

    let insert_guild_users = sqlx::query(indoc! {"
        INSERT INTO guild_users (guild_id, user_id, opted_out)
        VALUES ($1, $2, $3)
        ON CONFLICT (guild_id, user_id) DO UPDATE SET opted_out = EXCLUDED.opted_out;
    "});

    match insert_guild_users
        .bind(guild_id.get() as i64)
        .bind(user.id.get() as i64)
        .bind(opted_out)
        .execute(executor)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => info!("successfully set opt-out status"),
        Ok(_) => {
            error!("no update was made to the guild_users table");
            return Err(OptOutSetError::NoUpdateMade);
        }
        Err(error) => {
            error!(%error, "failed to set opt-out in guild users");
            return Err(OptOutSetError::UnexpectedSqlx {
                user_id: user.id,
                guild_id,
                error,
            });
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum OptOutSetError {
    #[error("no update was made to the guild_users table")]
    NoUpdateMade,

    #[error(
        "unexpected SQLx error when setting opt-out for user {user_id} in guild {guild_id}: \
         {error}"
    )]
    UnexpectedSqlx {
        user_id: UserId,
        guild_id: GuildId,
        #[source]
        error: SqlxError,
    },
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
        INSERT INTO guild_users (guild_id, user_id, opted_out)
        VALUES ($1, $2, false)
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
