-- Add up migration script here
CREATE TABLE "matches" (
  "id" uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  "overlay_id" uuid,
  "tournament_slug" VARCHAR NOT NULL,
  "team_a" VARCHAR NOT NULL,
  "team_b" VARCHAR NOT NULL,
  "team_a_score" INTEGER NOT NULL DEFAULT 0,
  "team_b_score" INTEGER NOT NULL DEFAULT 0,
  "completed" BOOLEAN NOT NULL DEFAULT false,
  "in_progress" BOOLEAN NOT NULL DEFAULT false,
  "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
