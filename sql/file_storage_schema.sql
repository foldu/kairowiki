CREATE TABLE file_hash (
    id INTEGER PRIMARY KEY NOT NULL,
    relative_path TEXT NOT NULL UNIQUE CHECK (
        relative_path <> ''
    ),
    hash BLOB NOT NULL UNIQUE CHECK (
        LENGTH(hash) = 32
    )
);

INSERT INTO migrations VALUES ('file_storage_schema');
