-- Trash tables for reversible transaction/trade deletion (UX guide §19, Undo).
--
-- Rationale: `transactions` is read by ~60 queries (balances, net worth,
-- portfolio joins, budgets, import dedup …). A `deleted_at` flag would require
-- every one of them to filter, and a single miss is a silent balance bug.
-- Instead, deleting physically MOVES the row (and its securities_trades child)
-- into these mirror tables; restore moves it back with the original id. No live
-- read query needs to change because the row is simply absent.
--
-- Columns mirror transactions / securities_trades 1:1 (plus deleted_at). No FK
-- constraints here — these are frozen copies. Keep in sync if those tables gain
-- columns (see move/restore SQL in db/transactions.rs, which lists columns).

CREATE TABLE IF NOT EXISTS deleted_transactions (
    id                INTEGER PRIMARY KEY,
    account_id        INTEGER NOT NULL,
    booking_date      TEXT    NOT NULL,
    value_date        TEXT,
    amount_cents      INTEGER NOT NULL,
    currency          TEXT    NOT NULL,
    counterparty      TEXT,
    purpose           TEXT,
    raw_ref           TEXT,
    category_id       INTEGER,
    source            TEXT    NOT NULL,
    source_file_hash  TEXT,
    imported_at       TEXT    NOT NULL,
    manual_note       TEXT,
    bucket_id         INTEGER,
    kind              TEXT    NOT NULL,
    counterparty_iban TEXT,
    paired_tx_id      INTEGER,
    deleted_at        TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS deleted_securities_trades (
    tx_id                 INTEGER PRIMARY KEY,
    security_id           INTEGER NOT NULL,
    side                  TEXT    NOT NULL,
    shares_micro          INTEGER NOT NULL,
    unit_price_micro      INTEGER,
    fee_cents             INTEGER NOT NULL DEFAULT 0,
    tax_cents             INTEGER NOT NULL DEFAULT 0,
    fx_rate_micro         INTEGER,
    account_id            INTEGER,
    kest_cents            INTEGER NOT NULL DEFAULT 0,
    withholding_tax_cents INTEGER NOT NULL DEFAULT 0,
    fusion_group          TEXT,
    deleted_at            TEXT    NOT NULL DEFAULT (datetime('now'))
);
