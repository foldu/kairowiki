CREATE TABLE wiki_user (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    pass_hash BLOB NOT NULL
);

INSERT INTO migrations VALUES ('user_schema');
