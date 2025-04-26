-- Add up migration script here
CREATE TABLE "scoreboard" (
  "overlay_id" uuid PRIMARY KEY,
  "team_a" VARCHAR NOT NULL,
  "team_a_score" INTEGER NOT NULL DEFAULT 0,
  "team_a_standing" VARCHAR,
  "team_b" VARCHAR NOT NULL,
  "team_b_score" INTEGER NOT NULL DEFAULT 0,
  "team_b_standing" VARCHAR
);
