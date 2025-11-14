-- Connect to postgres database to create other databases
\c postgres

-- Create bitbridge database if it doesn't exist
SELECT 'CREATE DATABASE bitbridge'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'bitbridge')\gexec

-- Create kong database if it doesn't exist
SELECT 'CREATE DATABASE kong'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'kong')\gexec

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE bitbridge TO bitbridge;
GRANT ALL PRIVILEGES ON DATABASE kong TO bitbridge;
