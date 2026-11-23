-- Spec 092: Enable automatic zero-click E2EE multi-device key & message sync by default
-- (Users can still opt into manual 6-digit device confirmation in Settings -> Storage -> Devices.)
ALTER TABLE device_storage_policies
    ALTER COLUMN require_device_approval SET DEFAULT FALSE;

UPDATE device_storage_policies
SET require_device_approval = FALSE
WHERE require_device_approval = TRUE;
