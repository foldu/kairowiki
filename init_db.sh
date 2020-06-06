#!/bin/sh
mkdir -p data/db
sqlite3 data/db/db.sqlite -init ./sql/migrations_schema.sql .exit
sqlite3 data/db/db.sqlite -init ./sql/user_schema.sql .exit
sqlite3 data/db/db.sqlite -init ./sql/file_storage_schema.sql .exit
