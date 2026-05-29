-- Envelope budgeting: dedicated allocation ledger, retire goals.
-- See spec 2026-05-29-buckets-envelope-budgeting-design.md.

-- 1. Allocation ledger. amount_cents is signed: + into bucket, - back to "unassigned".
CREATE TABLE bucket_allocations (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_id    INTEGER NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    amount_cents INTEGER NOT NULL,
    occurred_on  TEXT    NOT NULL,        -- YYYY-MM-DD
    note         TEXT,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX idx_bucket_allocations_bucket ON bucket_allocations(bucket_id);

-- 2. Preserve existing bucket balances losslessly.
--    Old balance = SUM(all tx tagged to bucket). New balance = SUM(allocations) +
--    SUM(tagged OUTFLOWS). So: seed one allocation per bucket = sum of the tagged
--    INFLOWS (amount_cents >= 0), then strip bucket_id from those inflows. Outflows
--    keep bucket_id and stay "Entnahmen". => new balance == old balance, exactly.
INSERT INTO bucket_allocations (bucket_id, amount_cents, occurred_on, note)
SELECT bucket_id,
       SUM(amount_cents),
       strftime('%Y-%m-%d','now'),
       'Migration: Anfangsbestand'
  FROM transactions
 WHERE bucket_id IS NOT NULL AND amount_cents >= 0
 GROUP BY bucket_id
HAVING SUM(amount_cents) <> 0;

UPDATE transactions
   SET bucket_id = NULL
 WHERE bucket_id IS NOT NULL AND amount_cents >= 0;

-- 3. Retire goals. No FK points at goals.id (verified), so a plain DROP is safe.
DROP INDEX IF EXISTS idx_goals_category;
DROP TABLE goals;
