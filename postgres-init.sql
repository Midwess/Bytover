-- Connect to postgres database to create other databases
\c postgres

-- Create app_gateway login role if it doesn't exist
SELECT 'CREATE ROLE app_gateway LOGIN PASSWORD ''appgatewaypass'''
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_gateway')\gexec

-- Create bitbridge database if it doesn't exist
SELECT 'CREATE DATABASE bitbridge'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'bitbridge')\gexec

-- Create kong database if it doesn't exist
SELECT 'CREATE DATABASE kong'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'kong')\gexec

-- Create app_gateway database owned by app_gateway if it doesn't exist
SELECT 'CREATE DATABASE app_gateway OWNER app_gateway'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'app_gateway')\gexec

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE bitbridge TO bitbridge;
GRANT ALL PRIVILEGES ON DATABASE kong TO bitbridge;
GRANT ALL PRIVILEGES ON DATABASE app_gateway TO app_gateway;
