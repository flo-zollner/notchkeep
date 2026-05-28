import type { Security, Holding, DividendEntry, PriceRow, TradeWithTx } from '$lib/api';
import { mulberry32 } from './timeseries';

/**
 * Synthetische Wertpapiere — keine echten Indizes/Anbieter im Namen
 * (kein MSCI/iShares/Vanguard/Xtrackers/Lyxor/S&P).
 * ISINs nach Schema `XX000DEMO####` — passt formal in ISIN-Regex,
 * ist aber sichtbar Demo-Daten.
 */
export const SEED_SECURITIES: Security[] = [
  {
    id: 1,
    isin: 'DE00DEMO0001',
    symbol: 'DMOEQ',
    name: 'Welt-Aktien Demo',
    currency: 'EUR',
    assetType: 'etf_equity',
    country: null,
    sector: null,
    note: null,
    archived: false,
    createdAt: '2024-01-15T10:00:00Z',
  },
  {
    id: 2,
    isin: 'DE00DEMO0002',
    symbol: 'DMOBND',
    name: 'Globale Anleihen Demo',
    currency: 'EUR',
    assetType: 'etf_bond',
    country: null,
    sector: null,
    note: null,
    archived: false,
    createdAt: '2024-02-01T10:00:00Z',
  },
  {
    id: 3,
    isin: 'DE00DEMO0003',
    symbol: 'DMOREIT',
    name: 'Immobilien-Demo REIT',
    currency: 'EUR',
    assetType: 'etf_reit',
    country: null,
    sector: 'Real Estate',
    note: null,
    archived: false,
    createdAt: '2024-04-12T10:00:00Z',
  },
  {
    id: 4,
    isin: 'DE00DEMO0004',
    symbol: 'BEISP',
    name: 'Beispiel-Industrie AG',
    currency: 'EUR',
    assetType: 'stock',
    country: 'DE',
    sector: 'Industrials',
    note: null,
    archived: false,
    createdAt: '2024-06-20T10:00:00Z',
  },
  {
    id: 5,
    isin: 'DE00DEMO0005',
    symbol: 'DIVDEM',
    name: 'Dividenden-Demo AG',
    currency: 'EUR',
    assetType: 'stock',
    country: 'DE',
    sector: 'Utilities',
    note: null,
    archived: false,
    createdAt: '2024-09-10T10:00:00Z',
  },
  {
    id: 6,
    isin: 'XF00DEMO0006',
    symbol: 'DMOCRY',
    name: 'Demo Token',
    currency: 'EUR',
    assetType: 'crypto',
    country: null,
    sector: null,
    note: null,
    archived: false,
    createdAt: '2025-02-01T10:00:00Z',
  },
];

/** Holdings als feste Tabelle. Werte in Cent/Micro. */
export const SEED_HOLDINGS: Holding[] = [
  {
    securityId: 1,
    isin: 'DE00DEMO0001',
    symbol: 'DMOEQ',
    name: 'Welt-Aktien Demo',
    currency: 'EUR',
    sharesMicro: 320_000_000, // 320 shares
    costBasisCents: 24_800_00,
    avgCostPerShareMicro: 77_500_000, // 77,50 EUR
    marketValueCents: 32_640_00,
    unrealizedCents: 7_840_00,
    lastPriceDate: '2026-05-27',
  },
  {
    securityId: 2,
    isin: 'DE00DEMO0002',
    symbol: 'DMOBND',
    name: 'Globale Anleihen Demo',
    currency: 'EUR',
    sharesMicro: 180_000_000,
    costBasisCents: 9_540_00,
    avgCostPerShareMicro: 53_000_000,
    marketValueCents: 9_360_00,
    unrealizedCents: -180_00,
    lastPriceDate: '2026-05-27',
  },
  {
    securityId: 3,
    isin: 'DE00DEMO0003',
    symbol: 'DMOREIT',
    name: 'Immobilien-Demo REIT',
    currency: 'EUR',
    sharesMicro: 95_000_000,
    costBasisCents: 4_180_00,
    avgCostPerShareMicro: 44_000_000,
    marketValueCents: 4_465_00,
    unrealizedCents: 285_00,
    lastPriceDate: '2026-05-27',
  },
  {
    securityId: 4,
    isin: 'DE00DEMO0004',
    symbol: 'BEISP',
    name: 'Beispiel-Industrie AG',
    currency: 'EUR',
    sharesMicro: 40_000_000,
    costBasisCents: 4_800_00,
    avgCostPerShareMicro: 120_000_000,
    marketValueCents: 5_440_00,
    unrealizedCents: 640_00,
    lastPriceDate: '2026-05-27',
  },
  {
    securityId: 5,
    isin: 'DE00DEMO0005',
    symbol: 'DIVDEM',
    name: 'Dividenden-Demo AG',
    currency: 'EUR',
    sharesMicro: 60_000_000,
    costBasisCents: 3_120_00,
    avgCostPerShareMicro: 52_000_000,
    marketValueCents: 3_480_00,
    unrealizedCents: 360_00,
    lastPriceDate: '2026-05-27',
  },
];

/** Dividenden-Historie über die letzten 18 Monate. */
export function generateDividends(): DividendEntry[] {
  const out: DividendEntry[] = [];
  let txId = 50_000;
  // Equity-ETF (sec 1): quarterly
  for (let q = 0; q < 6; q++) {
    const month = 1 + q * 3;
    const year = 2024 + Math.floor((month - 1) / 12);
    const actualMonth = ((month - 1) % 12) + 1;
    out.push({
      txId: txId++,
      bookingDate: `${year}-${String(actualMonth).padStart(2, '0')}-15`,
      securityId: 1,
      securityName: 'Welt-Aktien Demo',
      amountCents: 110_00 + q * 8_00,
      taxCents: 27_00,
    });
  }
  // Dividenden-Aktie (sec 5): yearly
  for (let y = 2024; y <= 2025; y++) {
    out.push({
      txId: txId++,
      bookingDate: `${y}-05-12`,
      securityId: 5,
      securityName: 'Dividenden-Demo AG',
      amountCents: 240_00,
      taxCents: 60_00,
    });
  }
  return out.sort((a, b) => b.bookingDate.localeCompare(a.bookingDate));
}

/** Trades als TradeWithTx — vereinfacht: monatliche Sparplan-Käufe in sec 1, einige Käufe in 2-5. */
export function generateTrades(): TradeWithTx[] {
  const out: TradeWithTx[] = [];
  let txId = 60_000;

  // Monatliche Sparplan-Käufe in sec 1 über 24 Monate
  for (let i = 0; i < 24; i++) {
    let m = 5 - i;
    let y = 2026;
    while (m <= 0) {
      m += 12;
      y -= 1;
    }
    const date = `${y}-${String(m).padStart(2, '0')}-05`;
    const rnd = mulberry32(y * 100 + m);
    const sharesMicro = 12_000_000 + Math.round(rnd() * 4_000_000);
    const unitPriceMicro = 70_000_000 + Math.round(rnd() * 12_000_000);
    const amountCents = -Math.round((sharesMicro * unitPriceMicro) / 1_000_000 / 10_000);
    out.push({
      trade: {
        txId,
        securityId: 1,
        side: 'buy',
        sharesMicro,
        unitPriceMicro,
        feeCents: 100,
        taxCents: 0,
        kestCents: 0,
        withholdingTaxCents: 0,
        fxRateMicro: null,
        accountId: 4, // Depot
      },
      tx: {
        id: txId,
        account_id: 3, // Verrechnungskonto
        booking_date: date,
        value_date: date,
        amount_cents: amountCents,
        currency: 'EUR',
        counterparty: 'Sparplan Demo',
        purpose: null,
        raw_ref: null,
        category_id: 8,
        source: 'mock',
        source_file_hash: null,
        imported_at: date + 'T10:00:00Z',
        manual_note: null,
        bucket_id: null,
        kind: 'buy',
        counterparty_iban: null,
        holding_account_id: 4,
        trade_side: 'buy',
        paired_tx_id: null,
      },
    });
    txId++;
  }

  // Einmal-Käufe in sec 2-5
  const oneOffs: { secId: number; date: string; sharesMicro: number; unitPriceMicro: number }[] = [
    { secId: 2, date: '2024-02-15', sharesMicro: 180_000_000, unitPriceMicro: 53_000_000 },
    { secId: 3, date: '2024-04-20', sharesMicro: 95_000_000, unitPriceMicro: 44_000_000 },
    { secId: 4, date: '2024-06-25', sharesMicro: 40_000_000, unitPriceMicro: 120_000_000 },
    { secId: 5, date: '2024-09-12', sharesMicro: 60_000_000, unitPriceMicro: 52_000_000 },
  ];
  for (const o of oneOffs) {
    const amountCents = -Math.round((o.sharesMicro * o.unitPriceMicro) / 1_000_000 / 10_000);
    out.push({
      trade: {
        txId,
        securityId: o.secId,
        side: 'buy',
        sharesMicro: o.sharesMicro,
        unitPriceMicro: o.unitPriceMicro,
        feeCents: 200,
        taxCents: 0,
        kestCents: 0,
        withholdingTaxCents: 0,
        fxRateMicro: null,
        accountId: 4,
      },
      tx: {
        id: txId,
        account_id: 3,
        booking_date: o.date,
        value_date: o.date,
        amount_cents: amountCents,
        currency: 'EUR',
        counterparty: 'Einmal-Kauf Demo',
        purpose: null,
        raw_ref: null,
        category_id: 8,
        source: 'mock',
        source_file_hash: null,
        imported_at: o.date + 'T10:00:00Z',
        manual_note: null,
        bucket_id: null,
        kind: 'buy',
        counterparty_iban: null,
        holding_account_id: 4,
        trade_side: 'buy',
        paired_tx_id: null,
      },
    });
    txId++;
  }

  return out.sort((a, b) => b.tx.booking_date.localeCompare(a.tx.booking_date));
}

/** Preis-History pro Security: 365 Tage zurück, ~0.05 %/Tag Drift mit Noise. */
export function generatePriceHistory(securityId: number, days = 365): PriceRow[] {
  const out: PriceRow[] = [];
  const today = new Date('2026-05-27');
  const sec = SEED_SECURITIES.find((s) => s.id === securityId);
  if (!sec) return out;
  const holding = SEED_HOLDINGS.find((h) => h.securityId === securityId);
  const endPrice =
    holding && holding.sharesMicro > 0
      ? Math.round((holding.marketValueCents / (holding.sharesMicro / 1_000_000)) * 10_000)
      : 80_000_000;

  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    const iso = d.toISOString().slice(0, 10);
    const rnd = mulberry32(securityId * 100_000 + i);
    const drift = Math.pow(1.0004, days - 1 - i); // gentle upward bias
    const noise = 0.99 + rnd() * 0.02;
    const closeMicro = Math.round((endPrice / drift) * noise);
    out.push({
      securityId,
      date: iso,
      closeMicro,
      source: 'mock',
    });
  }
  return out;
}
