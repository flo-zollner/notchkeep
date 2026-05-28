import type {
  Goal,
  GoalProgress,
  Bucket,
  BucketProgress,
  BucketRule,
  RecurringPayment,
  RecurringOverview,
  DetectedRecurring,
  Rule,
  Category,
  CurrencyStatus,
  CategoryMonthBudget,
  BudgetEntry,
  AppConfigInfo,
  IntegrityReport,
  PathCheckResult,
  BackupResult,
  BackupValidation,
  ExportResult,
  ImportReport,
  SyncConflictFile,
  NewGoalPayload,
  UpdateGoalPayload,
  NewBucketPayload,
  UpdateBucketPayload,
  NewBucketRulePayload,
  NewRecurringPayload,
  UpdateRecurringPayload,
  NewRulePayload,
  NewCategoryPayload,
} from '$lib/api';
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';

// ─── Goals ───
function goalHandlers(store: MockStore): HandlerRegistry {
  return {
    list_goals: (raw) => {
      const { includeArchived } = (raw ?? {}) as { includeArchived?: boolean };
      const src = includeArchived ? store.goals : store.goals.filter((g) => !g.archived);
      return src.map((g) => ({ ...g }));
    },
    get_goal: (raw) => {
      const { id } = raw as { id: number };
      const g = store.goals.find((x) => x.id === id);
      if (!g) throw { message: `Goal ${id} not found` };
      return { ...g };
    },
    create_goal: (raw) => {
      const { payload } = raw as { payload: NewGoalPayload };
      const g: Goal = {
        id: store.nextGoalId++,
        name: payload.name,
        categoryId: payload.categoryId,
        targetCents: payload.targetCents,
        startDate: payload.startDate ?? new Date().toISOString().slice(0, 10),
        targetDate: payload.targetDate ?? null,
        icon: payload.icon ?? null,
        color: payload.color ?? null,
        note: payload.note ?? null,
        archived: false,
        createdAt: new Date().toISOString(),
      };
      store.goals.push(g);
      return { ...g };
    },
    update_goal: (raw) => {
      const { id, payload } = raw as { id: number; payload: UpdateGoalPayload };
      const idx = store.goals.findIndex((g) => g.id === id);
      if (idx === -1) throw { message: `Goal ${id} not found` };
      store.goals[idx] = { ...store.goals[idx], ...payload };
      return { ...store.goals[idx] };
    },
    delete_goal: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.goals.findIndex((g) => g.id === id);
      if (idx === -1) return false;
      store.goals.splice(idx, 1);
      return true;
    },
    goal_progress: (raw) => {
      const { id } = raw as { id: number };
      const g = store.goals.find((x) => x.id === id);
      if (!g) throw { message: `Goal ${id} not found` };
      const result: GoalProgress = {
        goalId: g.id,
        currentCents: Math.round(g.targetCents * 0.42),
        monthlyAvgCents: Math.round(g.targetCents / 24),
        forecastDate: null,
        onTrack: true,
      };
      return result;
    },
    list_goal_progress: (raw) => {
      const { includeArchived } = (raw ?? {}) as { includeArchived?: boolean };
      const src = includeArchived ? store.goals : store.goals.filter((g) => !g.archived);
      return src.map((g): GoalProgress => ({
        goalId: g.id,
        currentCents: Math.round(g.targetCents * 0.42),
        monthlyAvgCents: Math.round(g.targetCents / 24),
        forecastDate: null,
        onTrack: true,
      }));
    },
  };
}

// ─── Buckets ───
function bucketHandlers(store: MockStore): HandlerRegistry {
  return {
    list_buckets: (raw) => {
      const { includeArchived } = (raw ?? {}) as { includeArchived?: boolean };
      const src = includeArchived ? store.buckets : store.buckets.filter((b) => !b.archived);
      return src.map((b) => ({ ...b }));
    },
    get_bucket: (raw) => {
      const { id } = raw as { id: number };
      const b = store.buckets.find((x) => x.id === id);
      if (!b) throw { message: `Bucket ${id} not found` };
      return { ...b };
    },
    create_bucket: (raw) => {
      const { payload } = raw as { payload: NewBucketPayload };
      const b: Bucket = {
        id: store.nextBucketId++,
        name: payload.name,
        icon: payload.icon ?? null,
        color: payload.color ?? null,
        note: payload.note ?? null,
        targetCents: payload.targetCents ?? null,
        startDate: payload.startDate ?? null,
        targetDate: payload.targetDate ?? null,
        archived: false,
        createdAt: new Date().toISOString(),
      };
      store.buckets.push(b);
      return { ...b };
    },
    update_bucket: (raw) => {
      const { id, payload } = raw as { id: number; payload: UpdateBucketPayload };
      const idx = store.buckets.findIndex((b) => b.id === id);
      if (idx === -1) throw { message: `Bucket ${id} not found` };
      store.buckets[idx] = { ...store.buckets[idx], ...payload };
      return { ...store.buckets[idx] };
    },
    delete_bucket: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.buckets.findIndex((b) => b.id === id);
      if (idx === -1) return false;
      store.buckets.splice(idx, 1);
      return true;
    },
    bucket_balance: () => 0,
    list_bucket_progress: () =>
      store.buckets
        .filter((b) => !b.archived)
        .map((b): BucketProgress => ({ bucketId: b.id, currentCents: 0, txCount: 0 })),
    bucket_monthly_flow: () => [],
  };
}

// ─── Bucket Rules ───
function bucketRuleHandlers(store: MockStore): HandlerRegistry {
  return {
    list_bucket_rules: () => store.bucketRules.map((r) => ({ ...r })),
    create_bucket_rule: (raw) => {
      const { payload } = raw as { payload: NewBucketRulePayload };
      const r: BucketRule = {
        id: store.nextBucketRuleId++,
        priority: payload.priority,
        name: payload.name,
        counterpartyContains: payload.counterpartyContains,
        minAmountCents: payload.minAmountCents,
        maxAmountCents: payload.maxAmountCents,
        targetBucketId: payload.targetBucketId,
        enabled: payload.enabled,
      };
      store.bucketRules.push(r);
      return r.id;
    },
    update_bucket_rule: (raw) => {
      const { rule } = raw as { rule: BucketRule };
      const idx = store.bucketRules.findIndex((r) => r.id === rule.id);
      if (idx !== -1) store.bucketRules[idx] = { ...rule };
      return null;
    },
    delete_bucket_rule: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.bucketRules.findIndex((r) => r.id === id);
      if (idx !== -1) store.bucketRules.splice(idx, 1);
      return null;
    },
    apply_bucket_rules_now: () => 0,
  };
}

// ─── Recurring ───

/**
 * Advance an ISO date (YYYY-MM-DD) by `n` periods of the given frequency.
 * Day-of-month is clamped to the target month's length (e.g. Jan 31 → Feb 28).
 */
function addPeriods(
  anchor: string,
  frequency: RecurringPayment['frequency'],
  n: number,
): string {
  const [y, m, d] = anchor.split('-').map(Number);
  if (frequency === 'weekly') {
    const dt = new Date(Date.UTC(y, m - 1, d + n * 7));
    return dt.toISOString().slice(0, 10);
  }
  const monthsToAdd = frequency === 'monthly' ? n : frequency === 'quarterly' ? n * 3 : n * 12;
  const total = m - 1 + monthsToAdd;
  const ny = y + Math.floor(total / 12);
  const nm = (total % 12) + 1;
  const daysInTargetMonth = new Date(Date.UTC(ny, nm, 0)).getUTCDate();
  const nd = Math.min(d, daysInTargetMonth);
  return `${ny}-${String(nm).padStart(2, '0')}-${String(nd).padStart(2, '0')}`;
}

function recurringHandlers(store: MockStore): HandlerRegistry {
  return {
    list_recurring: (raw) => {
      const { includeArchived } = (raw ?? {}) as { includeArchived?: boolean };
      const src = includeArchived
        ? store.recurring
        : store.recurring.filter((r) => !r.archived);
      return src.map((r) => ({ ...r }));
    },
    get_recurring: (raw) => {
      const { id } = raw as { id: number };
      const r = store.recurring.find((x) => x.id === id);
      if (!r) throw { message: `Recurring ${id} not found` };
      return { ...r };
    },
    create_recurring: (raw) => {
      const { payload } = raw as { payload: NewRecurringPayload };
      const r: RecurringPayment = {
        id: store.nextRecurringId++,
        name: payload.name,
        accountId: payload.accountId,
        categoryId: payload.categoryId ?? null,
        amountCents: payload.amountCents,
        frequency: payload.frequency,
        anchorDate: payload.anchorDate,
        counterparty: payload.counterparty ?? null,
        note: payload.note ?? null,
        archived: false,
        createdAt: new Date().toISOString(),
      };
      store.recurring.push(r);
      return { ...r };
    },
    update_recurring: (raw) => {
      const { id, payload } = raw as { id: number; payload: UpdateRecurringPayload };
      const idx = store.recurring.findIndex((r) => r.id === id);
      if (idx === -1) throw { message: `Recurring ${id} not found` };
      store.recurring[idx] = { ...store.recurring[idx], ...payload };
      return { ...store.recurring[idx] };
    },
    delete_recurring: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.recurring.findIndex((r) => r.id === id);
      if (idx === -1) return false;
      store.recurring.splice(idx, 1);
      return true;
    },
    recurring_overview: (raw) => {
      const { monthsAhead } = raw as { monthsAhead: number };
      return store.recurring
        .filter((r) => !r.archived)
        .map((r): RecurringOverview => ({
          recurring: { ...r },
          occurrences: Array.from({ length: monthsAhead }, (_, i) => ({
            // Advance the anchor by `i` periods so each occurrence has a distinct
            // due date — the real backend never returns two occurrences on the
            // same day, and the upcoming list keys on (id, dueDate).
            dueDate: addPeriods(r.anchorDate, r.frequency, i),
            status: i === 0 ? 'paid' : ('pending' as const),
            matchedTxId: null,
            matchedAmountCents: null,
          })),
        }));
    },
    detect_recurring: (): DetectedRecurring[] => [],
  };
}

// ─── Rules ───
function ruleHandlers(store: MockStore): HandlerRegistry {
  return {
    list_rules: () =>
      store.rules.map((r) => ({ ...r, conditions: r.conditions.map((c) => ({ ...c })) })),
    create_rule: (raw) => {
      const { rule } = raw as { rule: NewRulePayload };
      const r: Rule = {
        id: store.nextRuleId++,
        priority: rule.priority,
        name: rule.name,
        combinator: rule.combinator,
        conditions: rule.conditions.map((c) => ({ ...c })),
        targetCategoryId: rule.targetCategoryId,
        enabled: rule.enabled,
      };
      store.rules.push(r);
      return { ...r, conditions: r.conditions.map((c) => ({ ...c })) };
    },
    update_rule: (raw) => {
      const { rule } = raw as { rule: Rule };
      const idx = store.rules.findIndex((r) => r.id === rule.id);
      if (idx !== -1)
        store.rules[idx] = { ...rule, conditions: rule.conditions.map((c) => ({ ...c })) };
      return { ...store.rules[idx] };
    },
    delete_rule: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.rules.findIndex((r) => r.id === id);
      if (idx !== -1) store.rules.splice(idx, 1);
      return null;
    },
    apply_rule_to_existing: () => 0,
    preview_rule_match: (raw) => {
      const { rule } = raw as { rule: NewRulePayload };
      // Simple heuristic: count txs whose counterparty matches any string-op condition value
      let count = 0;
      for (const tx of store.transactions) {
        let match = rule.combinator === 'and';
        for (const c of rule.conditions) {
          const field =
            c.field === 'counterparty'
              ? tx.counterparty
              : c.field === 'description'
                ? tx.purpose
                : null;
          if (typeof field !== 'string') {
            if (rule.combinator === 'and') {
              match = false;
              break;
            }
            continue;
          }
          const hit = field.toLowerCase().includes(c.value.toLowerCase());
          if (rule.combinator === 'and') match = match && hit;
          else match = match || hit;
        }
        if (match) count++;
      }
      return count;
    },
  };
}

// ─── Categories (CRUD extras) ───
function categoryCrudHandlers(store: MockStore): HandlerRegistry {
  return {
    create_category: (raw) => {
      const { cat } = raw as { cat: NewCategoryPayload };
      const c: Category = {
        id: store.nextCategoryId++,
        parent_id: cat.parentId ?? null,
        name: cat.name,
        color: cat.color ?? null,
        icon: cat.icon ?? null,
        rollover_enabled: cat.rolloverEnabled,
      };
      store.categories.push(c);
      return { ...c };
    },
    update_category: (raw) => {
      const { cat } = raw as { cat: Category };
      const idx = store.categories.findIndex((c) => c.id === cat.id);
      if (idx !== -1) store.categories[idx] = { ...cat };
      return null;
    },
    delete_category: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.categories.findIndex((c) => c.id === id);
      if (idx !== -1) store.categories.splice(idx, 1);
      return null;
    },
    merge_categories: (raw) => {
      const { fromId, toId } = raw as { fromId: number; toId: number };
      let moved = 0;
      for (const tx of store.transactions) {
        if (tx.category_id === fromId) {
          tx.category_id = toId;
          moved++;
        }
      }
      return moved;
    },
  };
}

// ─── Currencies ───
function currencyHandlers(store: MockStore): HandlerRegistry {
  return {
    list_currencies: () => store.currencies.map((c) => ({ ...c })),
    update_currency_rate: (raw) => {
      const { currency, rateMicro } = raw as { currency: string; rateMicro: number };
      const c = store.currencies.find((x) => x.code === currency);
      if (c) {
        c.rateMicro = rateMicro;
        c.date = new Date().toISOString().slice(0, 10);
        c.source = 'manual';
      }
      return c ? { ...c } : ({ code: currency, rateMicro, date: null, source: null, inUse: false } as CurrencyStatus);
    },
    refresh_currency_rate: (raw) => {
      const { currency } = raw as { currency: string };
      const c = store.currencies.find((x) => x.code === currency);
      if (c) {
        c.date = new Date().toISOString().slice(0, 10);
        c.source = 'mock';
      }
      return c ? { ...c } : ({ code: currency, rateMicro: null, date: null, source: null, inUse: false } as CurrencyStatus);
    },
  };
}

// ─── Budgets ───
function budgetHandlers(store: MockStore): HandlerRegistry {
  function key(catId: number, year: number, month: number): string {
    return `${catId}|${year}|${month}`;
  }
  return {
    set_budget: (raw) => {
      const { categoryId, year, month, amountCents } = raw as {
        categoryId: number;
        year: number;
        month: number;
        amountCents: number;
      };
      store.budgets.set(key(categoryId, year, month), {
        categoryId,
        year,
        month,
        amountCents,
        createdAt: new Date().toISOString(),
      });
      return null;
    },
    clear_budget: (raw) => {
      const { categoryId, year, month } = raw as {
        categoryId: number;
        year: number;
        month: number;
      };
      return store.budgets.delete(key(categoryId, year, month));
    },
    list_budget_overrides: (raw) => {
      const { categoryId } = raw as { categoryId: number };
      const out: BudgetEntry[] = [];
      for (const [, v] of store.budgets) if (v.categoryId === categoryId) out.push({ ...v });
      return out;
    },
    month_overview: (raw) => {
      const { year, month } = raw as { year: number; month: number };
      const ym = `${year}-${String(month).padStart(2, '0')}`;

      // Resolve any category to its top-level ancestor so child spending rolls
      // up to the budgeted parent — mirrors the real backend's month_overview.
      const parentOf = new Map(store.categories.map((c) => [c.id, c.parent_id]));
      const topLevelOf = (id: number): number => {
        let cur = id;
        for (let guard = 0; parentOf.get(cur) != null && guard < 20; guard++) {
          cur = parentOf.get(cur) as number;
        }
        return cur;
      };

      const spentByTop = new Map<number, number>();
      for (const t of store.transactions) {
        if (t.category_id == null || t.amount_cents >= 0) continue; // expenses only
        if (!t.booking_date.startsWith(ym)) continue;
        const top = topLevelOf(t.category_id);
        spentByTop.set(top, (spentByTop.get(top) ?? 0) - t.amount_cents);
      }

      const result: CategoryMonthBudget[] = [];
      for (const c of store.categories.filter((cat) => cat.parent_id === null)) {
        const override = store.budgets.get(key(c.id, year, month));
        result.push({
          categoryId: c.id,
          categoryName: c.name,
          budgetCents: override?.amountCents ?? null,
          overrideCents: override?.amountCents ?? null,
          spentCents: spentByTop.get(c.id) ?? 0,
          rolloverCents: 0,
          rolloverEnabled: c.rollover_enabled,
        });
      }
      return result;
    },
  };
}

// ─── DataMgmt + Import/Export (Stubs) ───
function miscHandlers(_store: MockStore): HandlerRegistry {
  return {
    get_data_path_info: (): AppConfigInfo => ({
      dbPath: '/mock/data/budget.sqlite',
      dbSizeBytes: 1_024_000,
      dbModifiedIso: new Date().toISOString(),
      lockHolder: null,
    }),
    check_target_path: (): PathCheckResult => ({ kind: 'empty' }),
    change_data_path: () => null,
    backup_database: (): BackupResult => ({ bytes: 1_024_000, durationMs: 12 }),
    validate_backup: (): BackupValidation => ({
      ok: true,
      schemaVersion: 1,
      rowCounts: {
        transactions: 0,
        accounts: 0,
        categories: 0,
        securities: 0,
        recurringPayments: 0,
      },
      error: null,
    }),
    restore_database: () => null,
    wipe_database: () => null,
    retry_startup: () => null,
    set_path_and_init: () => null,
    reset_path_to_default: () => null,
    find_data_issues: (): IntegrityReport => ({
      tradeKindWithoutTradeRow: [],
      allocationsToArchivedBuckets: [],
      zombieSecurities: [],
    }),
    force_acquire_sync_lock: () => null,
    check_sync_conflicts: (): SyncConflictFile[] => [],
    resolve_conflict_keep_current: () => null,
    resolve_conflict_use_other: () => null,

    // Import stubs — produce a no-op report
    import_trade_republic_csv: (): ImportReport => ({
      parsed: 0,
      inserted: 0,
      skipped: 0,
      categorized_by_rule: 0,
      categorized_by_fuzzy: 0,
      warnings: ['Mock-Modus: kein echter Import'],
    }),
    import_flatex_pdfs: (): ImportReport => ({
      parsed: 0,
      inserted: 0,
      skipped: 0,
      categorized_by_rule: 0,
      categorized_by_fuzzy: 0,
      warnings: ['Mock-Modus: kein echter Import'],
    }),
    import_sparkasse_csv: (): ImportReport => ({
      parsed: 0,
      inserted: 0,
      skipped: 0,
      categorized_by_rule: 0,
      categorized_by_fuzzy: 0,
      warnings: ['Mock-Modus: kein echter Import'],
    }),

    export_transactions_csv: (): ExportResult => ({ rows: 0, bytes: 0 }),

    // Institutions CRUD extras
    create_institution: (raw) => {
      const { payload } = raw as { payload: { name: string } };
      return { id: 999, ...payload, archived: false, createdAt: new Date().toISOString() };
    },
    update_institution: (raw) => {
      const { id, payload } = raw as { id: number; payload: Record<string, unknown> };
      return { id, ...payload };
    },
    delete_institution: () => null,
  };
}

export function createExtraHandlers(store: MockStore): HandlerRegistry {
  return {
    ...goalHandlers(store),
    ...bucketHandlers(store),
    ...bucketRuleHandlers(store),
    ...recurringHandlers(store),
    ...ruleHandlers(store),
    ...categoryCrudHandlers(store),
    ...currencyHandlers(store),
    ...budgetHandlers(store),
    ...miscHandlers(store),
  };
}
