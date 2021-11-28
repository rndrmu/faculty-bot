-- Your SQL goes here
CREATE TABLE verification (
    id SERIAL PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    user_tag VARCHAR(255) NOT NULL,
    user_email VARCHAR(255) NOT NULL,
)