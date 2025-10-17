CREATE TABLE "user" (
    id UUID PRIMARY KEY,
    name TEXT,
    email TEXT UNIQUE NOT NULL,
    sub TEXT UNIQUE NOT NULL,
    picture TEXT
);