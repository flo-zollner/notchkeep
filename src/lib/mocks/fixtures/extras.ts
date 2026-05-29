import type { BucketAllocation, Bucket, RecurringPayment, Rule, CurrencyStatus, BucketRule } from '$lib/api';

export const SEED_BUCKETS: Bucket[] = [
  {
    id: 1,
    name: 'Auto',
    icon: 'car',
    color: 'var(--c3)',
    note: null,
    targetCents: null,
    startDate: null,
    targetDate: null,
    archived: false,
    createdAt: '2025-01-15T10:00:00Z',
  },
  {
    id: 2,
    name: 'Wohnung',
    icon: 'home',
    color: 'var(--c2)',
    note: null,
    targetCents: null,
    startDate: null,
    targetDate: null,
    archived: false,
    createdAt: '2025-01-15T10:00:00Z',
  },
  {
    id: 3,
    name: 'Reisen',
    icon: 'plane',
    color: 'var(--c5)',
    note: null,
    targetCents: 2_500_00,
    startDate: '2026-01-01',
    targetDate: '2026-08-01',
    archived: false,
    createdAt: '2026-01-01T10:00:00Z',
  },
];

export const SEED_ALLOCATIONS: BucketAllocation[] = [
  {
    id: 1,
    bucketId: 1,
    amountCents: 30_000,
    occurredOn: '2026-04-01',
    note: null,
    createdAt: '2026-04-01T10:00:00Z',
  },
  {
    id: 2,
    bucketId: 2,
    amountCents: 50_000,
    occurredOn: '2026-04-01',
    note: 'Miete Rücklage',
    createdAt: '2026-04-01T10:00:00Z',
  },
  {
    id: 3,
    bucketId: 3,
    amountCents: 20_000,
    occurredOn: '2026-05-01',
    note: null,
    createdAt: '2026-05-01T10:00:00Z',
  },
];

export const SEED_RECURRING: RecurringPayment[] = [
  {
    id: 1,
    name: 'Miete',
    accountId: 1,
    categoryId: 21,
    amountCents: -128_000,
    frequency: 'monthly',
    anchorDate: '2024-06-01',
    counterparty: 'Vermieter Mustermann GmbH',
    note: null,
    archived: false,
    createdAt: '2024-06-01T10:00:00Z',
  },
  {
    id: 2,
    name: 'Strom',
    accountId: 1,
    categoryId: 22,
    amountCents: -8_900,
    frequency: 'monthly',
    anchorDate: '2024-06-15',
    counterparty: 'Stadtwerke',
    note: null,
    archived: false,
    createdAt: '2024-06-15T10:00:00Z',
  },
  {
    id: 3,
    name: 'Streaming',
    accountId: 1,
    categoryId: 6,
    amountCents: -1_499,
    frequency: 'monthly',
    anchorDate: '2024-07-05',
    counterparty: 'Streaming-Abo',
    note: null,
    archived: false,
    createdAt: '2024-07-05T10:00:00Z',
  },
];

export const SEED_RULES: Rule[] = [
  {
    id: 1,
    priority: 0,
    name: 'Supermarkt → Lebensmittel',
    combinator: 'and',
    conditions: [{ field: 'counterparty', op: 'contains', value: 'Supermarkt' }],
    targetCategoryId: 11,
    enabled: true,
  },
  {
    id: 2,
    priority: 1,
    name: 'Tankstelle → Tanken',
    combinator: 'and',
    conditions: [{ field: 'counterparty', op: 'contains', value: 'Tankstelle' }],
    targetCategoryId: 32,
    enabled: true,
  },
];

export const SEED_BUCKET_RULES: BucketRule[] = [
  {
    id: 1,
    priority: 0,
    name: 'Tankstelle → Auto',
    counterpartyContains: 'Tankstelle',
    minAmountCents: null,
    maxAmountCents: null,
    targetBucketId: 1,
    enabled: true,
  },
];

export const SEED_CURRENCIES: CurrencyStatus[] = [
  { code: 'EUR', rateMicro: 1_000_000, date: '2026-05-27', source: 'mock', inUse: true },
  { code: 'USD', rateMicro: 1_090_000, date: '2026-05-27', source: 'mock', inUse: false },
  { code: 'GBP', rateMicro: 850_000, date: '2026-05-27', source: 'mock', inUse: false },
];
