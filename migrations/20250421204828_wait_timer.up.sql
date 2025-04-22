-- Add up migration script here
CREATE TABLE "wait_timer" (
  "overlay_id" uuid PRIMARY KEY,
  "wait_type" VARCHAR NOT NULL,
  "wait_until" VARCHAR NOT NULL
);
