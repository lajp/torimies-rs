-- This file should undo anything in `up.sql`
CREATE TABLE Blacklists(
    id INTEGER PRIMARY KEY NOT NULL,
    user_id INTEGER NOT NULL,
    seller_id INTEGER NOT NULL
);
