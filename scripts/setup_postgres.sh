#!/bin/sh
mkdir -p /run/postgresql
chown -R postgres:postgres /run/postgresql

CONF="/var/lib/postgresql/data/pgdata/postgresql.conf"
HBA="/var/lib/postgresql/data/pgdata/pg_hba.conf"

echo "port = 55432" >> "$CONF"
echo "listen_addresses = '*'" >> "$CONF"
echo "host all all all trust" >> "$HBA"

su - postgres -c "pg_ctl -D /var/lib/postgresql/data/pgdata -l /var/lib/postgresql/logfile start"
sleep 2

su - postgres -c "psql -p 55432 -c 'CREATE USER chatw WITH PASSWORD '\''chatw_dev_password'\'' SUPERUSER;'"
su - postgres -c "psql -p 55432 -c 'CREATE DATABASE chatw OWNER chatw;'"

su - postgres -c "psql -p 55432 -d chatw -U chatw -f /mnt/host/e/DATA/source/chatw/backend/migrations/0001_init.sql"
su - postgres -c "psql -p 55432 -d chatw -U chatw -f /mnt/host/e/DATA/source/chatw/backend/migrations/0002_avatar.sql"
