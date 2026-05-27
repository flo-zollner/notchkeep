-- Initial consolidated schema. Beträge IMMER in Cent (INTEGER), keine Floats.
-- Konsolidiert aus 20 inkrementellen Migrations (0001-0020) zu einer Single-Source-of-Truth.

CREATE TABLE accounts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    currency     TEXT    NOT NULL DEFAULT 'EUR',
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, icon TEXT NULL, color TEXT NULL, note TEXT NULL, last4 TEXT NULL
    CHECK (last4 IS NULL OR (length(last4) = 4 AND last4 GLOB '[0-9][0-9][0-9][0-9]')), archived INTEGER NOT NULL DEFAULT 0
    CHECK (archived IN (0, 1)), parent_id INTEGER NULL
    REFERENCES accounts(id) ON DELETE SET NULL, iban TEXT NULL
    CHECK (iban IS NULL OR (
        length(iban) BETWEEN 15 AND 34
        AND iban GLOB '[A-Z][A-Z][0-9][0-9]*'
    )), institution_id INTEGER NULL
    REFERENCES institutions(id) ON DELETE SET NULL);
CREATE TABLE categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id  INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    name       TEXT    NOT NULL,
    color      TEXT,
    icon       TEXT, rollover_enabled INTEGER NOT NULL DEFAULT 0
    CHECK (rollover_enabled IN (0, 1)),
    UNIQUE(parent_id, name)
);
INSERT INTO categories VALUES(1,NULL,'Lebensmittel','#10b981','utensils',0);
INSERT INTO categories VALUES(2,NULL,'Wohnen','#0ea5e9','home',0);
INSERT INTO categories VALUES(3,NULL,'Transport','#6366f1','car',0);
INSERT INTO categories VALUES(4,NULL,'Freizeit','#f59e0b','film',0);
INSERT INTO categories VALUES(5,NULL,'Gesundheit','#ef4444','heart',0);
INSERT INTO categories VALUES(6,NULL,'Versicherungen','#64748b','shield',0);
INSERT INTO categories VALUES(7,NULL,'Abonnements','#a855f7','repeat',0);
INSERT INTO categories VALUES(8,NULL,'Einkäufe','#ec4899','bag',0);
INSERT INTO categories VALUES(9,NULL,'Reisen','#06b6d4','plane',0);
INSERT INTO categories VALUES(10,NULL,'Einkommen','#22c55e','briefcase',0);
INSERT INTO categories VALUES(11,NULL,'Sonstiges','#94a3b8','tag',0);
CREATE TABLE rules (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    priority           INTEGER NOT NULL DEFAULT 100,
    name               TEXT    NOT NULL,
    target_category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    enabled            INTEGER NOT NULL DEFAULT 1
, combinator TEXT NOT NULL DEFAULT 'and');
CREATE TABLE sync_lock (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    device_id    TEXT    NOT NULL,
    hostname     TEXT    NOT NULL,
    acquired_at  TEXT    NOT NULL
);
CREATE TABLE rule_conditions (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id   INTEGER NOT NULL REFERENCES rules(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    field     TEXT    NOT NULL,
    op        TEXT    NOT NULL,
    value     TEXT    NOT NULL,
    UNIQUE(rule_id, position)
);
CREATE TABLE goals (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    category_id     INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    target_cents    INTEGER NOT NULL CHECK (target_cents > 0),
    start_date      TEXT    NOT NULL,
    target_date     TEXT,
    icon            TEXT,
    color           TEXT,
    note            TEXT,
    archived        INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE buckets (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    icon         TEXT    NULL,
    color        TEXT    NULL,
    note         TEXT    NULL,
    target_cents INTEGER NULL,
    start_date   TEXT    NULL,
    target_date  TEXT    NULL,
    archived     INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE securities (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    isin       TEXT    NOT NULL UNIQUE,
    symbol     TEXT,
    name       TEXT    NOT NULL,
    currency   TEXT    NOT NULL DEFAULT 'EUR',
    asset_type TEXT    NOT NULL,
    country    TEXT,
    sector     TEXT,
    note       TEXT,
    archived   INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (length(isin) = 12),
    CHECK (isin GLOB '[A-Z][A-Z][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][A-Z0-9][0-9]'),
    CHECK (asset_type IN ('stock','etf_equity','etf_bond','etf_reit','bond','crypto','other'))
);
CREATE TABLE security_prices (
    security_id INTEGER NOT NULL REFERENCES securities(id) ON DELETE CASCADE,
    date        TEXT    NOT NULL,
    close_micro INTEGER NOT NULL,
    source      TEXT    NOT NULL,
    PRIMARY KEY (security_id, date),
    CHECK (source IN ('yahoo','manual','tr_csv'))
);
CREATE TABLE security_breakdowns (
    security_id INTEGER NOT NULL REFERENCES securities(id) ON DELETE CASCADE,
    dimension   TEXT    NOT NULL,
    key         TEXT    NOT NULL,
    weight_bps  INTEGER NOT NULL,
    PRIMARY KEY (security_id, dimension, key),
    CHECK (dimension IN ('country','sector')),
    CHECK (weight_bps BETWEEN 0 AND 10000)
);
CREATE TABLE fx_rates (
    currency   TEXT    NOT NULL,
    date       TEXT    NOT NULL,
    rate_micro INTEGER NOT NULL,
    source     TEXT    NOT NULL,
    PRIMARY KEY (currency, date),
    CHECK (source IN ('yahoo','ecb','manual'))
);
CREATE TABLE category_budgets (
    category_id   INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    year          INTEGER NOT NULL,
    month         INTEGER NOT NULL CHECK (month BETWEEN 1 AND 12),
    amount_cents  INTEGER NOT NULL CHECK (amount_cents >= 0),
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (category_id, year, month)
);
CREATE TABLE recurring_payments (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    category_id     INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    amount_cents    INTEGER NOT NULL CHECK (amount_cents != 0),
    frequency       TEXT    NOT NULL CHECK (frequency IN ('weekly','monthly','quarterly','yearly')),
    anchor_date     TEXT    NOT NULL,
    counterparty    TEXT,
    note            TEXT,
    archived        INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE security_bucket_allocations (
    id           INTEGER PRIMARY KEY,
    security_id  INTEGER NOT NULL REFERENCES securities(id) ON DELETE CASCADE,
    bucket_id    INTEGER NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    shares_micro INTEGER NOT NULL CHECK(shares_micro > 0),
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(security_id, bucket_id)
);
CREATE TABLE bucket_rules (
    id                    INTEGER PRIMARY KEY,
    priority              INTEGER NOT NULL DEFAULT 100,
    name                  TEXT NOT NULL,
    counterparty_contains TEXT,
    min_amount_cents      INTEGER,
    max_amount_cents      INTEGER,
    target_bucket_id      INTEGER NOT NULL REFERENCES buckets(id) ON DELETE CASCADE,
    enabled               INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE institutions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    icon        TEXT    NULL,
    color       TEXT    NULL,
    bic         TEXT    NULL,
    country     TEXT    NULL,
    note        TEXT    NULL,
    archived    INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (bic IS NULL OR (length(bic) IN (8, 11) AND bic GLOB '[A-Z0-9]*')),
    CHECK (country IS NULL OR (length(country) = 2 AND country GLOB '[A-Z][A-Z]'))
);
CREATE TABLE IF NOT EXISTS "transactions" (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id        INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    booking_date      TEXT    NOT NULL,
    value_date        TEXT,
    amount_cents      INTEGER NOT NULL,
    currency          TEXT    NOT NULL DEFAULT 'EUR',
    counterparty      TEXT,
    purpose           TEXT,
    raw_ref           TEXT,
    category_id       INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    source            TEXT    NOT NULL,
    source_file_hash  TEXT,
    imported_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    manual_note       TEXT,
    bucket_id         INTEGER REFERENCES buckets(id) ON DELETE SET NULL,
    kind              TEXT    NOT NULL DEFAULT 'expense',
    counterparty_iban TEXT, paired_tx_id INTEGER REFERENCES transactions(id) ON DELETE SET NULL,
    CHECK (kind IN ('income','expense','transfer','buy','sell','dividend','corporate_action','tax','tax_general','fee'))
);
CREATE TABLE IF NOT EXISTS "securities_trades" (
    tx_id                 INTEGER PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
    security_id           INTEGER NOT NULL REFERENCES securities(id) ON DELETE RESTRICT,
    side                  TEXT    NOT NULL,
    shares_micro          INTEGER NOT NULL,
    unit_price_micro      INTEGER,
    fee_cents             INTEGER NOT NULL DEFAULT 0,
    tax_cents             INTEGER NOT NULL DEFAULT 0,
    fx_rate_micro         INTEGER,
    account_id            INTEGER REFERENCES accounts(id) ON DELETE SET NULL,
    kest_cents            INTEGER NOT NULL DEFAULT 0,
    withholding_tax_cents INTEGER NOT NULL DEFAULT 0,
    fusion_group          TEXT,
    CHECK (side IN ('buy','sell','dividend','corporate_action','fusion_out','fusion_in','tax'))
);
CREATE INDEX idx_rules_priority ON rules(priority) WHERE enabled = 1;
CREATE INDEX idx_rule_conditions_rule ON rule_conditions(rule_id);
CREATE INDEX idx_goals_category ON goals(category_id);
CREATE INDEX idx_accounts_parent ON accounts(parent_id);
CREATE UNIQUE INDEX idx_accounts_iban ON accounts(iban) WHERE iban IS NOT NULL;
CREATE INDEX idx_recurring_active ON recurring_payments(archived);
CREATE INDEX idx_sba_security ON security_bucket_allocations(security_id);
CREATE INDEX idx_sba_bucket   ON security_bucket_allocations(bucket_id);
CREATE INDEX idx_bucket_rules_target ON bucket_rules(target_bucket_id);
CREATE INDEX idx_bucket_rules_priority ON bucket_rules(priority);
CREATE UNIQUE INDEX idx_institutions_name_unique ON institutions(LOWER(name));
CREATE UNIQUE INDEX idx_institutions_bic_unique  ON institutions(bic) WHERE bic IS NOT NULL;
CREATE INDEX idx_accounts_institution ON accounts(institution_id);
CREATE INDEX idx_transactions_account_date ON transactions(account_id, booking_date);
CREATE INDEX idx_transactions_category     ON transactions(category_id);
CREATE INDEX idx_transactions_source_hash  ON transactions(source_file_hash);
CREATE INDEX idx_transactions_kind         ON transactions(kind);
CREATE UNIQUE INDEX idx_transactions_dedup ON transactions(
    account_id, booking_date, amount_cents, COALESCE(counterparty, ''), COALESCE(source_file_hash, '')
);
CREATE INDEX idx_securities_trades_security ON securities_trades(security_id);
CREATE INDEX idx_securities_trades_account  ON securities_trades(account_id);
CREATE INDEX idx_transactions_paired_tx ON transactions(paired_tx_id) WHERE paired_tx_id IS NOT NULL;
