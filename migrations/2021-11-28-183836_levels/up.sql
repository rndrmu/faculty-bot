-- Your SQL goes here
CREATE TABLE xp (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    xp INTEGER NOT NULL,
    current_lvl INTEGER NOT NULL,
)