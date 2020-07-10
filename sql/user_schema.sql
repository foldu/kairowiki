CREATE TABLE wiki_user (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE CHECK (
        name <> ''
    ),
    email TEXT NOT NULL UNIQUE CHECK (
        email <> ''
    ),
    pass_hash BLOB NOT NULL CHECK (
        LENGTH(pass_hash) = 40
    )
);

INSERT INTO migrations VALUES ('user_schema');
