-- Add up migration script here
CREATE TABLE "stream_overlay" (
  "id" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  "tournament_slug" VARCHAR NOT NULL,
  "name" VARCHAR,
  "team_a" VARCHAR,
  "team_b" VARCHAR,
  "score_a" VARCHAR,
  "score_b" VARCHAR
);
