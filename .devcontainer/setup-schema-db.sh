#!/bin/bash
# Creates an SQLite database which follows the same schema as the Wordbase
# database, and sets up `sqlx` to use this database for schema lookups.

DEV_DB_PATH="$HOME/wordbase-schema.sqlite"

if [ -f "$DEV_DB_PATH" ]; then
    rm "$DEV_DB_PATH"
fi
sqlite3 "$DEV_DB_PATH" < crates/wordbase-server/src/setup_db.sql
echo "DATABASE_URL=sqlite:$DEV_DB_PATH" > .env
