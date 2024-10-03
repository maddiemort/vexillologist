CREATE TABLE IF NOT EXISTS scores (
    guild_id BIGINT NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    correct INTEGER NOT NULL,
    board INTEGER NOT NULL,
    score REAL NOT NULL,
    rank INTEGER NOT NULL,
    players INTEGER NOT NULL,
    day_added INTEGER NOT NULL,
    UNIQUE (guild_id, user_id, board)
);
