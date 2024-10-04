use std::fmt;

use sqlx::FromRow;

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
