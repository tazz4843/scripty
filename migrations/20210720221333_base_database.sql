-- create prefix table
CREATE TABLE IF NOT EXISTS prefixes (
  guild_id BIGINT PRIMARY KEY,
  prefix TEXT
);

-- create guilds table
CREATE TABLE IF NOT EXISTS guilds (
  guild_id BIGINT PRIMARY KEY,
  default_bind BIGINT,
  output_channel BIGINT,
  premium_level SMALLINT NOT NULL
);

-- create users table
CREATE TABLE IF NOT EXISTS users (
  user_id BIGINT PRIMARY KEY,
  premium_level SMALLINT,
  premium_count SMALLINT
);

-- create channels table
CREATE TABLE IF NOT EXISTS channels (
  channel_id BIGINT PRIMARY KEY,
  webhook_token TEXT,
  webhook_id BIGINT
);

-- create api key table
CREATE TABLE IF NOT EXISTS api_keys (
    user_id BIGINT PRIMARY KEY,
    api_key TEXT NOT NULL
);
