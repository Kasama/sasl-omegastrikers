-- Add up migration script here
ALTER TABLE scoreboard
ADD logo VARCHAR NOT NULL DEFAULT '/assets/amongtitans.png';
