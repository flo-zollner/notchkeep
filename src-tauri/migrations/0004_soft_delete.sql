-- Soft-delete support: a non-null deleted_at hides the row from all reads;
-- restore clears it. Used by the Undo-Snackbar flow (UX guide §19).
ALTER TABLE buckets            ADD COLUMN deleted_at TEXT;
ALTER TABLE recurring_payments ADD COLUMN deleted_at TEXT;
ALTER TABLE rules              ADD COLUMN deleted_at TEXT;
