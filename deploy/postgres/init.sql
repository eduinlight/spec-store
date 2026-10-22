CREATE ROLE platform LOGIN PASSWORD :'platform_password';
CREATE DATABASE platform OWNER platform;
CREATE ROLE keycloak LOGIN PASSWORD :'keycloak_password';
CREATE DATABASE keycloak OWNER keycloak;
ALTER ROLE platform CREATEDB;
