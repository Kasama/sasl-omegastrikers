-- Add up migration script here
CREATE TABLE "team" (
  "id" VARCHAR PRIMARY KEY,
  "tournament_slug" VARCHAR NOT NULL,
  "name" VARCHAR NOT NULL,
  "nickname" VARCHAR,
  "image" VARCHAR
);
