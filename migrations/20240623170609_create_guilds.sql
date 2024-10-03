CREATE TABLE IF NOT EXISTS guilds (guild_id BIGINT PRIMARY KEY NOT NULL);

CREATE TABLE IF NOT EXISTS guild_users (
    guild_id BIGINT NOT NULL REFERENCES guilds (guild_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users (user_id) ON DELETE CASCADE,
    UNIQUE (guild_id, user_id)
);
