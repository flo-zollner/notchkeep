import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from '../index';
import type {
  Bucket,
  BucketAllocation,
  RecurringPayment,
  Rule,
  Category,
  CurrencyStatus,
  CategoryMonthBudget,
  AppConfigInfo,
  IntegrityReport,
  ExportResult,
  ImportReport,
  BucketRule,
  BucketProgress,
  RecurringOverview,
} from '$lib/api';

describe('Phase 4d — restliche Domänen', () => {
  // Buckets
  it('list_buckets + create_bucket + bucket_balance', async () => {
    const invoke = createMockTauriInvoke();
    const before = await invoke<Bucket[]>('list_buckets', { includeArchived: false });

    const created = await invoke<Bucket>('create_bucket', {
      payload: { name: 'Urlaub' },
    });
    expect(created.name).toBe('Urlaub');

    const balance = await invoke<number>('bucket_balance', { id: created.id });
    expect(typeof balance).toBe('number');

    const after = await invoke<Bucket[]>('list_buckets', { includeArchived: false });
    expect(after.length).toBe(before.length + 1);
  });

  it('list_bucket_progress returns BucketProgress[]', async () => {
    const invoke = createMockTauriInvoke();
    const progress = await invoke<BucketProgress[]>('list_bucket_progress');
    expect(Array.isArray(progress)).toBe(true);
  });

  it('ready_to_assign returns a number', async () => {
    const invoke = createMockTauriInvoke();
    const rta = await invoke<number>('ready_to_assign');
    expect(typeof rta).toBe('number');
  });

  it('list_bucket_allocations + create_bucket_allocation round-trip', async () => {
    const invoke = createMockTauriInvoke();
    const before = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 1 });

    const created = await invoke<BucketAllocation>('create_bucket_allocation', {
      payload: { bucketId: 1, amountCents: 5_000, occurredOn: '2026-05-01', note: 'test' },
    });
    expect(created.bucketId).toBe(1);
    expect(created.amountCents).toBe(5_000);

    const after = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 1 });
    expect(after.length).toBe(before.length + 1);
  });

  it('move_between_buckets creates two allocation entries', async () => {
    const invoke = createMockTauriInvoke();
    const before1 = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 1 });
    const before2 = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 2 });

    await invoke<void>('move_between_buckets', {
      fromBucket: 1, toBucket: 2, amountCents: 1_000, occurredOn: '2026-05-15',
    });

    const after1 = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 1 });
    const after2 = await invoke<BucketAllocation[]>('list_bucket_allocations', { bucketId: 2 });
    expect(after1.length).toBe(before1.length + 1);
    expect(after2.length).toBe(before2.length + 1);
  });

  // Bucket Rules
  it('list_bucket_rules + create_bucket_rule', async () => {
    const invoke = createMockTauriInvoke();
    await invoke<number>('create_bucket_rule', {
      payload: {
        priority: 0,
        name: 'Tanken→Auto',
        counterpartyContains: 'Tankstelle',
        minAmountCents: null,
        maxAmountCents: null,
        targetBucketId: 1,
        enabled: true,
      },
    });
    const rules = await invoke<BucketRule[]>('list_bucket_rules');
    expect(rules.length).toBeGreaterThan(0);
  });

  // Recurring
  it('list_recurring + create_recurring + recurring_overview', async () => {
    const invoke = createMockTauriInvoke();
    await invoke<RecurringPayment>('create_recurring', {
      payload: {
        name: 'Miete',
        accountId: 1,
        amountCents: -128_000,
        frequency: 'monthly',
        anchorDate: '2026-01-01',
      },
    });
    const list = await invoke<RecurringPayment[]>('list_recurring', { includeArchived: false });
    expect(list.length).toBeGreaterThan(0);

    const overview = await invoke<RecurringOverview[]>('recurring_overview', { monthsAhead: 3 });
    expect(Array.isArray(overview)).toBe(true);
  });

  // Rules
  it('list_rules + create_rule + preview_rule_match', async () => {
    const invoke = createMockTauriInvoke();
    const created = await invoke<Rule>('create_rule', {
      rule: {
        priority: 0,
        name: 'Supermarkt → Lebensmittel',
        combinator: 'and',
        conditions: [{ field: 'counterparty', op: 'contains', value: 'Supermarkt' }],
        targetCategoryId: 1,
        enabled: true,
      },
    });
    expect(created.name).toBe('Supermarkt → Lebensmittel');

    const matches = await invoke<number>('preview_rule_match', {
      rule: {
        priority: 0,
        name: 'preview',
        combinator: 'and',
        conditions: [{ field: 'counterparty', op: 'contains', value: 'Supermarkt' }],
        targetCategoryId: 1,
        enabled: true,
      },
    });
    expect(matches).toBeGreaterThan(0);

    const list = await invoke<Rule[]>('list_rules');
    expect(list.length).toBeGreaterThan(0);
  });

  // Categories CRUD
  it('create_category appends and merge_categories returns affected tx count', async () => {
    const invoke = createMockTauriInvoke();
    const created = await invoke<Category>('create_category', {
      cat: { name: 'Test-Kategorie', rolloverEnabled: false },
    });
    expect(created.name).toBe('Test-Kategorie');

    const moved = await invoke<number>('merge_categories', { fromId: created.id, toId: 1 });
    expect(typeof moved).toBe('number');
  });

  // Currencies
  it('list_currencies returns at least EUR', async () => {
    const invoke = createMockTauriInvoke();
    const cur = await invoke<CurrencyStatus[]>('list_currencies');
    expect(cur.some((c) => c.code === 'EUR')).toBe(true);
  });

  // Budgets
  it('set_budget + month_overview reflects budget', async () => {
    const invoke = createMockTauriInvoke();
    await invoke('set_budget', { categoryId: 1, year: 2026, month: 5, amountCents: 480_00 });
    const overview = await invoke<CategoryMonthBudget[]>('month_overview', { year: 2026, month: 5 });
    expect(overview.length).toBeGreaterThan(0);
    const cat1 = overview.find((c) => c.categoryId === 1);
    expect(cat1?.overrideCents).toBe(480_00);
  });

  // DataMgmt
  it('get_data_path_info returns AppConfigInfo shape', async () => {
    const invoke = createMockTauriInvoke();
    const info = await invoke<AppConfigInfo>('get_data_path_info');
    expect(info.dbPath).toBeTruthy();
    expect(typeof info.dbSizeBytes).toBe('number');
  });

  it('find_data_issues returns IntegrityReport shape', async () => {
    const invoke = createMockTauriInvoke();
    const rep = await invoke<IntegrityReport>('find_data_issues');
    expect(Array.isArray(rep.tradeKindWithoutTradeRow)).toBe(true);
    expect(Array.isArray(rep.allocationsToArchivedBuckets)).toBe(true);
    expect(Array.isArray(rep.zombieSecurities)).toBe(true);
  });

  // Export
  it('export_transactions_csv returns ExportResult shape', async () => {
    const invoke = createMockTauriInvoke();
    const r = await invoke<ExportResult>('export_transactions_csv', {
      filter: {},
      targetPath: '/tmp/mock.csv',
    });
    expect(typeof r.rows).toBe('number');
    expect(typeof r.bytes).toBe('number');
  });

  // Import (Stubs)
  it('import_sparkasse_csv returns ImportReport stub', async () => {
    const invoke = createMockTauriInvoke();
    const r = await invoke<ImportReport>('import_sparkasse_csv', {
      accountId: 1,
      bytes: [1, 2, 3],
    });
    expect(typeof r.parsed).toBe('number');
    expect(Array.isArray(r.warnings)).toBe(true);
  });
});
