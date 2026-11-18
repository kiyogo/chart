-- Spec 007: new server permission bits (kick members = 128, mention @everyone = 256,
-- connect to voice = 512). Existing @everyone roles keep voice access.
UPDATE server_roles SET permissions = permissions | 512 WHERE name = '@everyone' AND (permissions & 512) = 0;
