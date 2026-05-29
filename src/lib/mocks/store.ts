import type {
  Account,
  Category,
  Institution,
  Transaction,
  Bucket,
  BucketAllocation,
  RecurringPayment,
  Rule,
  BucketRule,
  CurrencyStatus,
  BudgetEntry,
} from '$lib/api';
import { SEED_ACCOUNTS } from './fixtures/accounts';
import { SEED_CATEGORIES } from './fixtures/categories';
import { SEED_INSTITUTIONS } from './fixtures/institutions';
import { SEED_TRANSACTIONS } from './fixtures/transactions';
import {
  SEED_BUCKETS,
  SEED_ALLOCATIONS,
  SEED_RECURRING,
  SEED_RULES,
  SEED_BUCKET_RULES,
  SEED_CURRENCIES,
} from './fixtures/extras';

/**
 * Mutable In-Memory-Store für die Mock-Schicht. Reset = Page-Reload (Vite-HMR
 * verwirft das Modul). Nicht persistiert.
 */
export interface MockStore {
  accounts: Account[];
  categories: Category[];
  institutions: Institution[];
  transactions: Transaction[];
  buckets: Bucket[];
  allocations: BucketAllocation[];
  recurring: RecurringPayment[];
  rules: Rule[];
  bucketRules: BucketRule[];
  currencies: CurrencyStatus[];
  /** Key: `${categoryId}|${year}|${month}` */
  budgets: Map<string, BudgetEntry>;
  nextAccountId: number;
  nextCategoryId: number;
  nextInstitutionId: number;
  nextTransactionId: number;
  nextAllocationId: number;
  nextBucketId: number;
  nextRecurringId: number;
  nextRuleId: number;
  nextBucketRuleId: number;
}

export function createMockStore(): MockStore {
  const accounts = SEED_ACCOUNTS.map((a) => ({ ...a }));
  const categories = SEED_CATEGORIES.map((c) => ({ ...c }));
  const institutions = SEED_INSTITUTIONS.map((i) => ({ ...i }));
  const transactions = SEED_TRANSACTIONS.map((t) => ({ ...t }));
  const buckets = SEED_BUCKETS.map((b) => ({ ...b }));
  const allocations = SEED_ALLOCATIONS.map((a) => ({ ...a }));
  const recurring = SEED_RECURRING.map((r) => ({ ...r }));
  const rules = SEED_RULES.map((r) => ({ ...r, conditions: r.conditions.map((c) => ({ ...c })) }));
  const bucketRules = SEED_BUCKET_RULES.map((r) => ({ ...r }));
  const currencies = SEED_CURRENCIES.map((c) => ({ ...c }));
  return {
    accounts,
    categories,
    institutions,
    transactions,
    buckets,
    allocations,
    recurring,
    rules,
    bucketRules,
    currencies,
    budgets: new Map(),
    nextAccountId: Math.max(...accounts.map((a) => a.id)) + 1,
    nextCategoryId: Math.max(...categories.map((c) => c.id)) + 1,
    nextInstitutionId: Math.max(...institutions.map((i) => i.id)) + 1,
    nextTransactionId: Math.max(...transactions.map((t) => t.id)) + 1,
    nextAllocationId: Math.max(...allocations.map((a) => a.id), 0) + 1,
    nextBucketId: Math.max(...buckets.map((b) => b.id), 0) + 1,
    nextRecurringId: Math.max(...recurring.map((r) => r.id), 0) + 1,
    nextRuleId: Math.max(...rules.map((r) => r.id), 0) + 1,
    nextBucketRuleId: Math.max(...bucketRules.map((r) => r.id), 0) + 1,
  };
}
