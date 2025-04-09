CREATE TABLE "users" (
  "id" SERIAL PRIMARY KEY,
  "username" text NOT NULL,
  "discord" text NOT NULL,
  "omegastrikers_id" text,
  "startgg_id" text,
  "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  UNIQUE("username")
);
