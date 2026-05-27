-- Standard-Kategorie "Investitionen" für Wertpapier-Käufe/-Verkäufe.
-- INSERT ist idempotent über die UNIQUE(parent_id, name)-Constraint, falls
-- der User die Kategorie schon manuell angelegt hat.
INSERT OR IGNORE INTO categories (parent_id, name, color, icon, rollover_enabled)
VALUES (NULL, 'Investitionen', '#8b5cf6', 'reports', 0);

-- Bestehende buy/sell-Tx ohne Kategorie auf die neue "Investitionen" setzen.
UPDATE transactions
   SET category_id = (
       SELECT id FROM categories
        WHERE parent_id IS NULL AND name = 'Investitionen' LIMIT 1
   )
 WHERE category_id IS NULL
   AND kind IN ('buy', 'sell');
