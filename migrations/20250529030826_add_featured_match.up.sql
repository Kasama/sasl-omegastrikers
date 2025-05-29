-- Add up migration script here
ALTER TABLE matches
ADD featured BOOLEAN NOT NULL DEFAULT FALSE;
