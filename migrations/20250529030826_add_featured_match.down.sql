-- Add down migration script here
ALTER TABLE matches
DROP COLUMN featured;
