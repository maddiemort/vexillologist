use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    id: i64,
    username: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct Guild {
    id: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct GuildUser {
    guild_id: i64,
    user_id: i64,
}
