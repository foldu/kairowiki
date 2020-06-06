CREATE TABLE file_hash (
    id INTEGER PRIMARY KEY NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    hash BLOB NOT NULL UNIQUE
);

INSERT INTO migrations VALUES ('file_storage_schema');
