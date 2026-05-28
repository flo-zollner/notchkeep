import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from '../index';
import type {
  Goal,
  Bucket,
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
  GoalProgress,
  BucketProgress,
  RecurringOverview,
} from '$lib/api';

describe('Phase 4d — restliche Domänen', () => {
  // Goals
  it('list_goals + create_goal + delete_goal round-trip', async () => {
    const invoke = createMockTauriInvoke();
    const before = await invoke<Goal[]>('list_goals', { includeArchived: false });

    const created = await invoke<Goal>('create_goal', {
      payload: { name: 'Notgroschen', categoryId: 1, targetCents: 10_000_00, startDate: '2026-01-01' },
    });
    expect(created.name).toBe('Notgroschen');

    const after = await invoke<Goal[]>('list_goals', { includeArchived: false });
    expect(after.length).toBe(before.length + 1);

    const ok = await invoke<boolean>('delete_goal', { id: created.id });
    expect(ok).toBe(true);
  });

  it('list_goal_progress returns one entry per goal', async () => {
    const invoke = createMockTauriInvoke();
    const goals = await invoke<Goal[]>('list_goals', { includeArchived: false });
    const progress = await invoke<GoalProgress[]>('list_goal_progress', { includeArchived: false });
    expect(progress).toHaveLength(goals.length);
  });

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
