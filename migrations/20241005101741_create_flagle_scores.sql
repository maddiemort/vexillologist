CREATE TABLE IF NOT EXISTS flagle_scores (
    guild_id BIGINT NOT NULL REFERENCES guilds (guild_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users (user_id) ON DELETE CASCADE,
    score INTEGER NOT NULL,
    board INTEGER NOT NULL,
    day_added INTEGER NOT NULL,
    UNIQUE (guild_id, user_id, board)
);
