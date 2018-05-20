CREATE TABLE lambdas (
  id SERIAL PRIMARY KEY,
  path TEXT NOT NULL,
  hostname TEXT NOT NULL,
  code TEXT NOT NULL,
  UNIQUE(hostname, path)
);
