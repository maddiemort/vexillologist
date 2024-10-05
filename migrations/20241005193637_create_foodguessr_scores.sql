CREATE TABLE IF NOT EXISTS foodguessr_scores (
    guild_id BIGINT NOT NULL REFERENCES guilds (guild_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users (user_id) ON DELETE CASCADE,
    score INTEGER NOT NULL,
    year INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    year_added INTEGER NOT NULL,
    ordinal_added INTEGER NOT NULL,
    UNIQUE (guild_id, user_id, year, ordinal)
);
