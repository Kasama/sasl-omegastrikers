-- Add up migration script here
CREATE TABLE "casters" (
  "overlay_id" VARCHAR NOT NULL,
  "kind" VARCHAR NOT NULL,
  "name" VARCHAR NOT NULL,
  "stream_video" VARCHAR NOT NULL,
  PRIMARY KEY (overlay_id, kind)
);
