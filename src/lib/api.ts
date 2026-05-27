import { invoke } from '@tauri-apps/api/core';
import { settings } from '$lib/settings.svelte';

export interface Account {
  id: number;
  name: string;
  kind: string;
  currency: string;
  icon: string | null;
  color: string | null;
  note: string | null;
  last4: string | null;
  archived: boolean;
  parent_id: number | null;
  iban: string | null;
  institution_id: number | null;
  created_at: string;
}

export interface Category {
  id: number;
  parent_id: number | null;
  name: string;
  color: string | null;
  icon: string | null;
  rollover_enabled: boolean;
}

export interface Transaction {
  id: number;
  account_id: number;
  booking_date: string;
  value_date: string | null;
  amount_cents: number;
  currency: string;
  counterparty: string | null;
  purpose: string | null;
  raw_ref: string | null;
  category_id: number | null;
  source: string;
  source_file_hash: string | null;
  imported_at: string;
  manual_note: string | null;
  bucket_id: number | null;
  kind: string;
  counterparty_iban: string | null;
  /** Depot/holding account for trade transactions (buy/sell/corp_action/fusion).
   *  Cash-only transactions and dividends/withholding tax return null. Evaluated
   *  by the account filter in addition to `account_id` → a buy transaction appears
   *  in both account views (cash and depot). */
  holding_account_id: number | null;
  /** `securities_trades.side` of the trade detail row. Allows the UI to
   *  distinguish merger debit (`fusion_out`) from merger credit (`fusion_in`) —
   *  both have `kind='corporate_action'` but different directions.
   *  `null` for cash transactions without a trade row. */
  trade_side: string | null;
  /** Pointer to the paired tx (inter-account transfer auto-pair).
   *  Null when the tx has no pair. */
  paired_tx_id: number | null;
}

export type MatchFieldId = 'counterparty' | 'description' | 'amount' | 'account';
export type MatchOpId =
  | 'contains'
  | 'equals'
  | 'starts_with'
  | 'ends_with'
  | 'regex'
  | 'range';
export type RuleCombinator = 'and' | 'or';

export interface RuleCondition {
  field: MatchFieldId;
  op: MatchOpId;
  /** Range: `"<min_cents>..<max_cents>"`. Account: `"<account_id>"`. Otherwise plain text. */
  value: string;
}

export interface Rule {
  id: number;
  priority: number;
  name: string;
  combinator: RuleCombinator;
  conditions: RuleCondition[];
  targetCategoryId: number;
  enabled: boolean;
}

export interface NewRulePayload {
  priority: number;
  name: string;
  combinator: RuleCombinator;
  conditions: RuleCondition[];
  targetCategoryId: number;
  enabled: boolean;
}

export interface BucketRule {
  id: number;
  priority: number;
  name: string;
  counterpartyContains: string | null;
  minAmountCents: number | null;
  maxAmountCents: number | null;
  targetBucketId: number;
  enabled: boolean;
}

export interface NewBucketRulePayload {
  priority: number;
  name: string;
  counterpartyContains: string | null;
  minAmountCents: number | null;
  maxAmountCents: number | null;
  targetBucketId: number;
  enabled: boolean;
}

export interface ImportReport {
  parsed: number;
  inserted: number;
  skipped: number;
  categorized_by_rule: number;
  categorized_by_fuzzy: number;
  warnings: string[];
}

export interface TxFilter {
  accountId?: number;
  institutionId?: number | null;
  categoryId?: number;
  bucketId?: number;
  search?: string;
  /** Inclusive (YYYY-MM-DD). Transactions with booking_date >= from. */
  from?: string;
  /** Inclusive (YYYY-MM-DD). Transactions with booking_date <= to. */
  to?: string;
  /** Only transactions without a category. */
  uncategorized?: boolean;
  /** abs(amount_cents) >= min. */
  minAmountCents?: number;
  /** Page size; default 200, clamped to [1, 5000]. */
  limit?: number;
  /** Opaque cursor 'YYYY-MM-DD|<id>'. */
  cursor?: string;
}

export interface ListTransactionsPage {
  rows: Transaction[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface TxAggregate {
  inCents: number;
  outCents: number;
  count: number;
}

export interface NewTransactionPayload {
  accountId: number;
  bookingDate: string; // YYYY-MM-DD
  amountCents: number;
  currency?: string;
  counterparty?: string | null;
  purpose?: string | null;
  categoryId?: number | null;
  bucketId?: number | null;
  manualNote?: string | null;
  counterpartyIban?: string | null;
  /** Optional. Allowed values: income, expense, transfer, fee. Trade kinds go
   *  through createTrade. Omitted = auto-derived from amount sign. */
  kind?: string;
}

export interface UpdateTransactionPayload {
  id: number;
  accountId: number;
  bookingDate: string;
  amountCents: number;
  currency: string;
  counterparty: string | null;
  purpose: string | null;
  categoryId: number | null;
  bucketId: number | null;
  manualNote: string | null;
  counterpartyIban: string | null;
  /** Optional override. Omitted = leave unchanged. */
  kind?: string;
}

export interface CategorySuggestion {
  categoryId: number;
  categoryName: string;
  score: number;
}

export interface CategorySpending {
  categoryId: number;
  spentCents: number;
}

export interface MonthlyFlow {
  year: number;
  month: number;
  inCents: number;
  outCents: number;
}

export interface CashflowSlice {
  categoryId: number | null;
  sign: 1 | -1;
  sumAbsCents: number;
}

export interface LockHolder {
  deviceId: string;
  hostname: string;
  acquiredAt: string;  // ISO 8601 UTC
}

export interface AppConfigInfo {
  dbPath: string;
  dbSizeBytes: number;
  dbModifiedIso: string;
  lockHolder: LockHolder | null;
}

export interface OrphanTradeTx {
  id: number;
  kind: string;
  bookingDate: string;
  counterparty: string | null;
  amountCents: number;
}

export interface AllocToArchivedBucket {
  allocationId: number;
  securityId: number;
  securityName: string;
  bucketId: number;
  bucketName: string;
  sharesMicro: number;
}

export interface ZombieSecurity {
  id: number;
  isin: string;
  name: string;
}

export interface IntegrityReport {
  tradeKindWithoutTradeRow: OrphanTradeTx[];
  allocationsToArchivedBuckets: AllocToArchivedBucket[];
  zombieSecurities: ZombieSecurity[];
}

export type PathCheckResult =
  | { kind: 'existing'; dbSizeBytes: number; valid: boolean }
  | { kind: 'empty' };

export interface BackupResult {
  bytes: number;
  durationMs: number;
}

export interface BackupRowCounts {
  transactions: number;
  accounts: number;
  categories: number;
  securities: number;
  recurringPayments: number;
}

export interface BackupValidation {
  ok: boolean;
  schemaVersion: number | null;
  rowCounts: BackupRowCounts;
  error: string | null;
}

export type ChangePathAction =
  | 'useExisting'
  | 'overwriteCopy'
  | 'move'
  | 'copy'
  | 'startFresh';

export interface SyncConflictFile {
  path: string;
  name: string;
  modifiedUnix: number;
}

export interface NetWorthPoint {
  year: number;
  month: number;
  totalCents: number;
}

export interface NetWorthForecastPoint {
  year: number;
  month: number;
  midCents: number;
  loCents: number;
  hiCents: number;
}

export interface NewCategoryPayload {
  name: string;
  parentId?: number | null;
  color?: string | null;
  icon?: string | null;
  rolloverEnabled: boolean;
}

export interface Goal {
  id: number;
  name: string;
  categoryId: number;
  targetCents: number;
  startDate: string;            // YYYY-MM-DD
  targetDate: string | null;
  icon: string | null;
  color: string | null;
  note: string | null;
  archived: boolean;
  createdAt: string;
}

export interface NewGoalPayload {
  name: string;
  categoryId: number;
  targetCents: number;
  startDate?: string;
  targetDate?: string | null;
  icon?: string | null;
  color?: string | null;
  note?: string | null;
}

export interface UpdateGoalPayload {
  name?: string;
  categoryId?: number;
  targetCents?: number;
  startDate?: string;
  targetDate?: string | null;
  icon?: string | null;
  color?: string | null;
  note?: string | null;
  archived?: boolean;
}

export interface GoalProgress {
  goalId: number;
  currentCents: number;
  monthlyAvgCents: number;
  forecastDate: string | null;  // YYYY-MM-01
  onTrack: boolean | null;
}

export interface SecurityBucketAllocation {
  id: number;
  securityId: number;
  bucketId: number;
  sharesMicro: number;
}

export interface BucketHoldingRow {
  securityId: number;
  securityName: string;
  isin: string;
  sharesMicro: number;
  valueCents: number;
}

export interface AllocationItem {
  bucketId: number;
  sharesMicro: number;
}

export interface Bucket {
  id: number;
  name: string;
  icon: string | null;
  color: string | null;
  note: string | null;
  targetCents: number | null;
  startDate: string | null;
  targetDate: string | null;
  archived: boolean;
  createdAt: string;
}

export interface NewBucketPayload {
  name: string;
  icon?: string | null;
  color?: string | null;
  note?: string | null;
  targetCents?: number | null;
  startDate?: string | null;
  targetDate?: string | null;
}

export interface UpdateBucketPayload {
  name?: string;
  icon?: string | null;
  color?: string | null;
  note?: string | null;
  targetCents?: number | null;
  startDate?: string | null;
  targetDate?: string | null;
  archived?: boolean;
}

export interface BucketProgress {
  bucketId: number;
  currentCents: number;
  txCount: number;
}

export type AssetType =
  | 'stock'
  | 'etf_equity'
  | 'etf_bond'
  | 'etf_reit'
  | 'bond'
  | 'crypto'
  | 'other';

export type BreakdownDimension = 'country' | 'sector';

export interface Security {
  id: number;
  isin: string;
  symbol: string | null;
  name: string;
  currency: string;
  assetType: AssetType;
  country: string | null;
  sector: string | null;
  note: string | null;
  archived: boolean;
  createdAt: string;
}

export interface NewSecurityPayload {
  isin: string;
  symbol: string | null;
  name: string;
  currency: string | null;
  assetType: AssetType;
  country: string | null;
  sector: string | null;
  note: string | null;
}

export interface UpdateSecurityPayload {
  isin?: string;
  symbol?: string | null;
  name?: string;
  currency?: string;
  assetType?: AssetType;
  country?: string | null;
  sector?: string | null;
  note?: string | null;
  archived?: boolean;
}

export interface SecurityBreakdown {
  securityId: number;
  dimension: BreakdownDimension;
  key: string;
  weightBps: number;
}

export interface BreakdownRowInput {
  key: string;
  weightBps: number;
}

export interface BudgetEntry {
  categoryId: number;
  year: number;
  month: number;
  amountCents: number;
  createdAt: string;
}

export interface CategoryMonthBudget {
  categoryId: number;
  categoryName: string;
  budgetCents: number | null;       // effective (forward-filled)
  overrideCents: number | null;     // explicit
  spentCents: number;
  rolloverCents: number;            // 0 in 6m1
  rolloverEnabled: boolean;
}

export interface InvestmentFlow {
  buysCents: number;
  sellsCents: number;
  dividendsCents: number;
  netInvestedCents: number;
}

export type TxKind =
  | 'income' | 'expense' | 'transfer'
  | 'buy' | 'sell' | 'dividend' | 'corporate_action';

export type TradeSide = 'buy' | 'sell' | 'dividend' | 'corporate_action' | 'tax';

export interface SecurityTrade {
  txId: number;
  securityId: number;
  side: TradeSide;
  sharesMicro: number;
  unitPriceMicro: number | null;
  feeCents: number;
  taxCents: number;
  kestCents: number;
  withholdingTaxCents: number;
  fxRateMicro: number | null;
  /** Optional explicit depot account. Null = falls back to tx.account_id. */
  accountId: number | null;
}

export interface TradeWithTx {
  trade: SecurityTrade;
  tx: Transaction;
}

export interface NewTradePayload {
  /** transactions.account_id (= cash account). Where the cash flow is recorded. */
  accountId: number;
  securityId: number;
  bookingDate: string;
  side: TradeSide;
  sharesMicro: number;
  unitPriceMicro: number | null;
  feeCents: number;
  taxCents: number;
  fxRateMicro: number | null;
  amountCents: number;
  currency: string | null;
  counterparty: string | null;
  manualNote: string | null;
  /** Optional override for securities_trades.account_id (= depot).
   *  If null/undefined: backend derives automatically (= sole broker account
   *  of the same institution via resolve_trade_account). */
  holdingAccountId?: number | null;
}

export interface UpdateTradePayload {
  sharesMicro?: number;
  unitPriceMicro?: number | null;
  feeCents?: number;
  taxCents?: number;
  fxRateMicro?: number | null;
  // Added for depot dialog:
  kestCents?: number;
  withholdingTaxCents?: number;
  /** securities_trades.account_id (= depot, when explicitly set). */
  accountId?: number;
  securityId?: number;
  amountCents?: number;
  /** transactions.account_id (= cash account). Disambiguates from accountId above. */
  txAccountId?: number;
}

export interface CurrencyStatus {
  code: string;
  rateMicro: number | null;
  date: string | null;
  source: string | null;
  inUse: boolean;
}

// ─── Portfolio ───

export interface Holding {
  securityId: number;
  isin: string;
  symbol: string | null;
  name: string;
  currency: string;
  sharesMicro: number;
  costBasisCents: number;
  avgCostPerShareMicro: number;
  marketValueCents: number;
  unrealizedCents: number;
  lastPriceDate: string | null;
}

export interface PriceRow {
  securityId: number;
  date: string;
  closeMicro: number;
  source: string;
}

export interface RefreshReport {
  securitiesTotal: number;
  pricesUpdated: number;
  pricesFailed: number;
  fxUpdated: number;
  fxFailed: number;
}

export interface DividendEntry {
  txId: number;
  bookingDate: string;
  securityId: number;
  securityName: string;
  amountCents: number;
  taxCents: number;
}

export interface CostBasisPoint {
  year: number;
  month: number;
  costBasisCents: number;
  marketValueCents: number;
}

export interface CostBasisPointDaily {
  date: string;          // YYYY-MM-DD
  costBasisCents: number;
  marketValueCents: number;
}

export interface AllocationSlice {
  key: string;
  valueCents: number;
}

export interface PortfolioKpis {
  marketValueCents: number;
  costBasisCents: number;
  unrealizedCents: number;
  realizedYtdCents: number;
}

export interface AccountMarketValue {
  accountId: number;
  marketValueCents: number;
}

export interface ExportFilter {
  from?: string;        // YYYY-MM-DD
  to?: string;          // YYYY-MM-DD
  accountId?: number;
  institutionId?: number;
  categoryId?: number;
  search?: string;
}

export interface ExportResult {
  rows: number;
  bytes: number;
}

// ─── Recurring Payments ───

export interface RecurringPayment {
  id: number;
  name: string;
  accountId: number;
  categoryId: number | null;
  amountCents: number;
  frequency: 'weekly' | 'monthly' | 'quarterly' | 'yearly';
  anchorDate: string;
  counterparty: string | null;
  note: string | null;
  archived: boolean;
  createdAt: string;
}

export interface NewRecurringPayload {
  name: string;
  accountId: number;
  categoryId?: number | null;
  amountCents: number;
  frequency: 'weekly' | 'monthly' | 'quarterly' | 'yearly';
  anchorDate: string;
  counterparty?: string | null;
  note?: string | null;
}

export interface UpdateRecurringPayload {
  name?: string;
  categoryId?: number | null;
  amountCents?: number;
  frequency?: 'weekly' | 'monthly' | 'quarterly' | 'yearly';
  anchorDate?: string;
  counterparty?: string | null;
  note?: string | null;
  archived?: boolean;
}

export interface Occurrence {
  dueDate: string;
  status: 'paid' | 'pending';
  matchedTxId: number | null;
  matchedAmountCents: number | null;
}

export interface RecurringOverview {
  recurring: RecurringPayment;
  occurrences: Occurrence[];
}

export interface DetectedRecurring {
  counterparty: string;
  accountId: number;
  amountCents: number;
  frequency: 'weekly' | 'monthly' | 'quarterly' | 'yearly';
  anchorDate: string;
  sampleCount: number;
}

/** Returns true when the transaction is a security trade with a trade detail row
 *  (buy/sell/dividend/corp-action/tax accumulation). Routing decision for the UI:
 *  cash transaction → TxModal, trade transaction → DepotTxModal.
 *  `fee` is intentionally excluded — a pure fee transaction is cash editing
 *  and goes through TxModal. */
export function isTradeTx(tx: Transaction): boolean {
  return ['buy', 'sell', 'dividend', 'corporate_action', 'tax'].includes(tx.kind);
}

export const api = {
  // Accounts
  listAccounts: () => invoke<Account[]>('get_accounts'),
  createAccount: (
    name: string,
    kind: string,
    currency?: string,
    parentId?: number | null,
    iban?: string | null,
    institutionId?: number | null,
  ) =>
    invoke<Account>('create_account', {
      name, kind, currency,
      parentId: parentId ?? null,
      iban: iban ?? null,
      institutionId: institutionId ?? null,
    }),
  getAccount: (id: number) => invoke<Account>('get_account', { id }),
  updateAccount: (account: Account) => invoke<void>('update_account', { account }),
  accountBalance: (id: number) => invoke<number>('account_balance', { id }),

  // Transactions
  listTransactions: (filter?: TxFilter) =>
    invoke<ListTransactionsPage>('list_transactions', { filter: filter ?? null }),
  aggregateTransactions: (filter?: TxFilter) =>
    invoke<TxAggregate>('aggregate_transactions', { filter: filter ?? null }),
  createTransaction: (tx: NewTransactionPayload) =>
    invoke<Transaction>('create_transaction', { tx }),
  updateTransaction: (tx: UpdateTransactionPayload) =>
    invoke<Transaction>('update_transaction', { tx }),
  deleteTransaction: (id: number) => invoke<void>('delete_transaction', { id }),
  detectTransfers: () => invoke<number>('detect_transfers'),
  cleanupPhantomMirrors: () => invoke<number>('cleanup_phantom_mirrors'),
  assignCategory: (transactionId: number, categoryId: number | null) =>
    invoke<void>('assign_category', { transactionId, categoryId }),
  assignAccount: (transactionId: number, accountId: number) =>
    invoke<void>('assign_account', { transactionId, accountId }),
  suggestCategory: (name: string, accountId?: number | null) =>
    invoke<CategorySuggestion | null>('suggest_category', {
      name,
      accountId: accountId ?? null,
    }),

  // Categories
  listCategories: () => invoke<Category[]>('list_categories'),
  createCategory: (cat: NewCategoryPayload) =>
    invoke<Category>('create_category', { cat }),
  updateCategory: (cat: Category) => invoke<void>('update_category', { cat }),
  deleteCategory: (id: number) => invoke<void>('delete_category', { id }),
  mergeCategories: (fromId: number, toId: number) =>
    invoke<number>('merge_categories', { fromId, toId }),

  // Rules
  listRules: () => invoke<Rule[]>('list_rules'),
  createRule: (rule: NewRulePayload) => invoke<Rule>('create_rule', { rule }),
  updateRule: (rule: Rule) => invoke<Rule>('update_rule', { rule }),
  deleteRule: (id: number) => invoke<void>('delete_rule', { id }),
  applyRuleToExisting: (ruleId: number) =>
    invoke<number>('apply_rule_to_existing', { ruleId }),
  previewRuleMatch: (rule: NewRulePayload) =>
    invoke<number>('preview_rule_match', { rule }),

  // Import
  importTradeRepublicCsv: (accountId: number, bytes: Uint8Array) =>
    invoke<ImportReport>('import_trade_republic_csv', {
      accountId,
      bytes: Array.from(bytes),
    }),
  importFlatexPdfs: (accountId: number, files: Uint8Array[]) =>
    invoke<ImportReport>('import_flatex_pdfs', {
      accountId,
      files: files.map((f) => Array.from(f)),
    }),
  importSparkasseCsv: (accountId: number, bytes: Uint8Array) =>
    invoke<ImportReport>('import_sparkasse_csv', {
      accountId,
      bytes: Array.from(bytes),
    }),

  // Aggregates
  monthlySpending: (year: number, month: number) =>
    invoke<CategorySpending[]>('monthly_spending', { year, month }),
  monthlyCashflow: (endYear: number, endMonth: number, months: number, excludeInvest?: boolean) =>
    invoke<MonthlyFlow[]>('monthly_cashflow', { endYear, endMonth, months, excludeInvest: excludeInvest ?? null }),
  categoryBreakdown: (from: string, to: string) =>
    invoke<CategorySpending[]>('category_breakdown', { from, to }),
  cashflowBreakdown: (from: string, to: string) =>
    invoke<CashflowSlice[]>('cashflow_breakdown', { from, to }),
  dailySpending: (year: number, month: number) =>
    invoke<number[]>('daily_spending', { year, month }),
  accountMonthlyCashflow: (accountId: number, endYear: number, endMonth: number, months: number) =>
    invoke<MonthlyFlow[]>('account_monthly_cashflow', { accountId, endYear, endMonth, months }),
  netWorthHistory: (endYear: number, endMonth: number, months: number) =>
    invoke<NetWorthPoint[]>('net_worth_history', { endYear, endMonth, months }),
  netWorthForecast: (
    endYear: number,
    endMonth: number,
    historyWindow: number,
    forecastMonths: number,
  ) =>
    invoke<NetWorthForecastPoint[]>('net_worth_forecast', {
      endYear,
      endMonth,
      historyWindow,
      forecastMonths,
    }),

  // Goals
  listGoals: (includeArchived = false) =>
    invoke<Goal[]>('list_goals', { includeArchived }),
  getGoal: (id: number) => invoke<Goal>('get_goal', { id }),
  createGoal: (payload: NewGoalPayload) =>
    invoke<Goal>('create_goal', { payload }),
  updateGoal: (id: number, payload: UpdateGoalPayload) =>
    invoke<Goal>('update_goal', { id, payload }),
  deleteGoal: (id: number) => invoke<boolean>('delete_goal', { id }),
  goalProgress: (id: number) =>
    invoke<GoalProgress>('goal_progress', { id }),
  listGoalProgress: (includeArchived = false) =>
    invoke<GoalProgress[]>('list_goal_progress', { includeArchived }),

  // Buckets
  listBuckets: (includeArchived = false) =>
    invoke<Bucket[]>('list_buckets', { includeArchived }),
  getBucket: (id: number) => invoke<Bucket>('get_bucket', { id }),
  createBucket: (payload: NewBucketPayload) =>
    invoke<Bucket>('create_bucket', { payload }),
  updateBucket: (id: number, payload: UpdateBucketPayload) =>
    invoke<Bucket>('update_bucket', { id, payload }),
  deleteBucket: (id: number) => invoke<boolean>('delete_bucket', { id }),
  bucketBalance: (id: number) => invoke<number>('bucket_balance', { id }),
  listBucketProgress: () =>
    invoke<BucketProgress[]>('list_bucket_progress'),
  assignBucket: (transactionId: number, bucketId: number | null) =>
    invoke<void>('assign_bucket', { transactionId, bucketId }),

  // Bucket Rules
  listBucketRules: () => invoke<BucketRule[]>('list_bucket_rules'),
  createBucketRule: (payload: NewBucketRulePayload) => invoke<number>('create_bucket_rule', { payload }),
  updateBucketRule: (rule: BucketRule) => invoke<void>('update_bucket_rule', { rule }),
  deleteBucketRule: (id: number) => invoke<void>('delete_bucket_rule', { id }),
  applyBucketRulesNow: (days: number) => invoke<number>('apply_bucket_rules_now', { days }),

  // Securities
  listSecurities: (includeArchived = false) =>
    invoke<Security[]>('list_securities', { includeArchived }),
  getSecurity: (id: number) => invoke<Security>('get_security', { id }),
  createSecurity: (payload: NewSecurityPayload) =>
    invoke<Security>('create_security', { payload }),
  updateSecurity: (id: number, payload: UpdateSecurityPayload) =>
    invoke<Security>('update_security', { id, payload }),
  deleteSecurity: (id: number) => invoke<boolean>('delete_security', { id }),

  // Breakdowns
  getBreakdown: (securityId: number, dimension: BreakdownDimension) =>
    invoke<SecurityBreakdown[]>('get_breakdown', { securityId, dimension }),
  setBreakdown: (
    securityId: number,
    dimension: BreakdownDimension,
    rows: BreakdownRowInput[],
  ) => invoke<null>('set_breakdown', { securityId, dimension, rows }),

  // Trades
  listTrades: (securityId?: number) =>
    invoke<TradeWithTx[]>('list_trades', { securityId: securityId ?? null }),
  getTrade: (txId: number) => invoke<TradeWithTx>('get_trade', { txId }),
  createTrade: (payload: NewTradePayload) =>
    invoke<TradeWithTx>('create_trade', { payload }),
  updateTrade: (txId: number, payload: UpdateTradePayload) =>
    invoke<SecurityTrade>('update_trade', { txId, payload }),
  deleteTrade: (txId: number) => invoke<boolean>('delete_trade', { txId }),

  // Prices
  refreshPrices: () => invoke<RefreshReport>('refresh_prices'),
  setManualPrice: (securityId: number, date: string, priceMicro: number) =>
    invoke<null>('set_manual_price', { securityId, date, priceMicro }),
  getPriceHistory: (securityId: number) =>
    invoke<PriceRow[]>('get_price_history', { securityId }),
  fetchSecurityHistory: (securityId: number, years: number) =>
    invoke<number>('fetch_security_history', { securityId, years }),

  // Portfolio
  listHoldings: () => invoke<Holding[]>('list_holdings'),
  assetAllocation: (dimension: 'asset_type' | 'country' | 'sector') =>
    invoke<AllocationSlice[]>('asset_allocation', { dimension }),
  realizedGainsSummary: (year: number | null) =>
    invoke<number>('realized_gains_summary', { year }),
  dividendHistory: () => invoke<DividendEntry[]>('dividend_history'),
  costBasisHistory: (endYear: number, endMonth: number, months: number) =>
    invoke<CostBasisPoint[]>('cost_basis_history', { endYear, endMonth, months }),
  costBasisHistoryDaily: (endDate: string, days: number) =>
    invoke<CostBasisPointDaily[]>('cost_basis_history_daily', { endDate, days }),
  portfolioKpis: (year: number) =>
    invoke<PortfolioKpis>('portfolio_kpis', { year }),
  portfolioValueByAccount: () =>
    invoke<AccountMarketValue[]>('portfolio_value_by_account_today'),

  // Security ↔ Bucket Allocations
  listSecurityAllocations: (securityId: number) =>
    invoke<SecurityBucketAllocation[]>('list_security_allocations', { securityId }),
  setSecurityAllocations: (securityId: number, items: AllocationItem[]) =>
    invoke<void>('set_security_allocations', { securityId, items }),
  bucketHoldings: (bucketId: number) =>
    invoke<BucketHoldingRow[]>('bucket_holdings', { bucketId }),

  // Monthly budgets
  setBudget: (categoryId: number, year: number, month: number, amountCents: number) =>
    invoke<null>('set_budget', { categoryId, year, month, amountCents }),
  clearBudget: (categoryId: number, year: number, month: number) =>
    invoke<boolean>('clear_budget', { categoryId, year, month }),
  listBudgetOverrides: (categoryId: number) =>
    invoke<BudgetEntry[]>('list_budget_overrides', { categoryId }),
  monthOverview: (year: number, month: number) =>
    invoke<CategoryMonthBudget[]>('month_overview', { year, month }),
  uncategorizedMonthlySpent: (year: number, month: number) =>
    invoke<number>('uncategorized_monthly_spent', { year, month }),
  investmentFlowForMonth: (year: number, month: number) =>
    invoke<InvestmentFlow>('investment_flow_for_month', { year, month }),

  // Export
  exportTransactionsCsv(filter: ExportFilter, targetPath: string): Promise<ExportResult> {
    return invoke<ExportResult>('export_transactions_csv', { filter, targetPath });
  },

  // Recurring
  listRecurring: (includeArchived: boolean = false) =>
    invoke<RecurringPayment[]>('list_recurring', { includeArchived }),
  getRecurring: (id: number) =>
    invoke<RecurringPayment>('get_recurring', { id }),
  createRecurring: (payload: NewRecurringPayload) =>
    invoke<RecurringPayment>('create_recurring', { payload }),
  updateRecurring: (id: number, payload: UpdateRecurringPayload) =>
    invoke<RecurringPayment>('update_recurring', { id, payload }),
  deleteRecurring: (id: number) =>
    invoke<boolean>('delete_recurring', { id }),
  recurringOverview: (monthsAhead: number) =>
    invoke<RecurringOverview[]>('recurring_overview', { monthsAhead }),
  detectRecurring: () =>
    invoke<DetectedRecurring[]>('detect_recurring'),

  // Currencies
  listCurrencies: () =>
    invoke<CurrencyStatus[]>('list_currencies'),
  updateCurrencyRate: (currency: string, rateMicro: number) =>
    invoke<CurrencyStatus>('update_currency_rate', { currency, rateMicro }),
  refreshCurrencyRate: (currency: string) =>
    invoke<CurrencyStatus>('refresh_currency_rate', { currency }),

  // Data management (7c)
  getDataPathInfo: () => invoke<AppConfigInfo>('get_data_path_info'),
  checkTargetPath: (targetDir: string) =>
    invoke<PathCheckResult>('check_target_path', { targetDir }),
  changeDataPath: (targetDir: string, action: ChangePathAction) =>
    invoke<void>('change_data_path', { targetDir, action }),
  backupDatabase: (targetPath: string) =>
    invoke<BackupResult>('backup_database', { targetPath }),
  validateBackup: (sourcePath: string) =>
    invoke<BackupValidation>('validate_backup', { sourcePath }),
  restoreDatabase: (sourcePath: string) =>
    invoke<void>('restore_database', { sourcePath }),
  wipeDatabase: () => invoke<void>('wipe_database'),
  retryStartup: () => invoke<void>('retry_startup'),
  setPathAndInit: (targetDir: string) => invoke<void>('set_path_and_init', { targetDir }),
  resetPathToDefault: () => invoke<void>('reset_path_to_default'),
  findDataIssues: () => invoke<IntegrityReport>('find_data_issues'),
  forceAcquireSyncLock: () => invoke<void>('force_acquire_sync_lock'),
  async checkSyncConflicts(): Promise<SyncConflictFile[]> {
    return await invoke<SyncConflictFile[]>('check_sync_conflicts');
  },
  async resolveConflictKeepCurrent(): Promise<void> {
    await invoke<void>('resolve_conflict_keep_current');
  },
  async resolveConflictUseOther(otherPath: string): Promise<void> {
    await invoke<void>('resolve_conflict_use_other', { otherPath });
  },
  bucketMonthlyFlow: (bucketId: number, endYear: number, endMonth: number, months: number) =>
    invoke<MonthlyFlow[]>('bucket_monthly_flow', { bucketId, endYear, endMonth, months }),
};

// ─── Institutions ───

export interface Institution {
  id: number;
  name: string;
  icon: string | null;
  color: string | null;
  bic: string | null;
  country: string | null;
  note: string | null;
  archived: boolean;
  createdAt: string;
}

export interface InstitutionSummary extends Institution {
  accountCount: number;
  balanceCents: number;
}

export interface NewInstitutionPayload {
  name: string;
  icon?: string | null;
  color?: string | null;
  bic?: string | null;
  country?: string | null;
  note?: string | null;
}

export interface UpdateInstitutionPayload {
  name?: string | null;
  icon?: string | null;
  color?: string | null;
  bic?: string | null;
  country?: string | null;
  note?: string | null;
  archived?: boolean | null;
}

export function listInstitutions(includeArchived = false): Promise<Institution[]> {
  return invoke('list_institutions', { includeArchived });
}

export function listInstitutionsWithSummary(): Promise<InstitutionSummary[]> {
  return invoke('list_institutions_with_summary');
}

export function getInstitution(id: number): Promise<Institution> {
  return invoke('get_institution', { id });
}

export function createInstitution(payload: NewInstitutionPayload): Promise<Institution> {
  return invoke('create_institution', { payload });
}

export function updateInstitution(id: number, payload: UpdateInstitutionPayload): Promise<Institution> {
  return invoke('update_institution', { id, payload });
}

export function deleteInstitution(id: number): Promise<void> {
  return invoke('delete_institution', { id });
}

// ─── Formatters ───

/**
 * Extracts a readable error message from a Tauri invoke rejection.
 * The backend throws `CommandError { message }` as an object — `String(e)` would
 * produce `[object Object]`. This function extracts `e.message`.
 */
export function errMsg(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object') {
    const o = e as Record<string, unknown>;
    if (typeof o.message === 'string') return o.message;
    try { return JSON.stringify(o); } catch { /* fall through */ }
  }
  return String(e);
}

export function fmtEur(cents: number, opts: { hide?: boolean; signed?: boolean; decimals?: number } = {}): string {
  if (opts.hide) return '•••• €';
  const decimals = opts.decimals ?? 2;
  const value = cents / 100;
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  const formatter = new Intl.NumberFormat(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
  const formatted = formatter.format(Math.abs(value));
  const sign = value < 0 ? '−' : opts.signed && value > 0 ? '+' : '';
  return `${sign}${formatted} €`;
}

/**
 * Parses a Euro string from input fields. Tolerantly accepts both locale formats:
 * '1.234,56', '1,234.56', '1234.56', '1234,56', '-12,5', '5'.
 * Returns NaN for invalid input (callers check themselves).
 */
export function parseEur(input: string): number {
  if (input == null) return NaN;
  const trimmed = String(input).trim();
  if (trimmed === '') return NaN;
  const lastComma = trimmed.lastIndexOf(',');
  const lastDot = trimmed.lastIndexOf('.');
  let normalized: string;
  if (lastComma > lastDot) {
    // Comma is decimal separator → dots are thousands separators (remove).
    normalized = trimmed.replace(/\./g, '').replace(',', '.');
  } else if (lastDot > lastComma) {
    // Dot is decimal separator → commas are thousands separators (remove).
    normalized = trimmed.replace(/,/g, '');
  } else {
    // No separator — input is an integer.
    normalized = trimmed;
  }
  return parseFloat(normalized);
}

/** Locale-specific decimal separator (for placeholders / help texts). */
export function decimalSep(): string {
  return settings.lang === 'en' ? '.' : ',';
}

/** Variant of `parseEur` that directly returns cents as a rounded integer.
 *  Returns 0 for invalid input. */
export function parseEurCents(input: string): number {
  const n = parseEur(input);
  return Number.isFinite(n) ? Math.round(n * 100) : 0;
}

/**
 * Formats a cent amount as a plain-number string for input fields
 * (locale decimal separator, no thousands grouping, no currency symbol).
 * Example: 12345 → "123,45" (de) or "123.45" (en).
 */
export function fmtEurInput(cents: number, decimals = 2): string {
  const value = Math.abs(cents) / 100;
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  return value.toLocaleString(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
    useGrouping: false,
  });
}

/**
 * Formats an arbitrary float as a locale-aware plain-number string
 * (e.g. for shares/quantities with variable decimal precision).
 * `decimals` omitted → natural precision (like Number.toString()).
 */
export function fmtNumInput(value: number, decimals?: number): string {
  if (!Number.isFinite(value)) return '';
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  if (decimals != null) {
    return value.toLocaleString(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
      useGrouping: false,
    });
  }
  // Natural representation: use toString then swap '.' for ',' on de-locale.
  const s = value.toString();
  return settings.lang === 'en' ? s : s.replace('.', ',');
}

export function fmtDate(iso: string, lang: 'de' | 'en' = 'de'): string {
  const parts = iso.split('-');
  if (parts.length !== 3) return iso;
  const [, m, d] = parts;
  const months = lang === 'de'
    ? ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez']
    : ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return `${parseInt(d, 10)}. ${months[parseInt(m, 10) - 1]}`;
}

export function todayIso(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${m}-${day}`;
}
