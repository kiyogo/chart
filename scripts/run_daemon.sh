#!/bin/sh
mkdir -p /var/lib/postgresql/data
mount -o loop /mnt/host/c/chatw_db/pg.img /var/lib/postgresql/data 2>/dev/null
mkdir -p /run/postgresql
chown -R postgres:postgres /run/postgresql /var/lib/postgresql
exec su - postgres -c "postgres -D /var/lib/postgresql/data/pgdata -p 55432 -h '*'"
