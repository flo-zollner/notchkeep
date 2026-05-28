import type { Institution } from '$lib/api';

/**
 * Synthetische Institute — keine echten Banken/Broker-Trademarks.
 * Erlaubt Komponenten wie InstitutionCard mit nicht-trivialen Daten zu rendern.
 */
export const SEED_INSTITUTIONS: Institution[] = [
  {
    id: 1,
    name: 'Beispielbank AG',
    icon: 'building-bank',
    color: 'var(--c2)',
    bic: 'BANKDEFFXXX',
    country: 'DE',
    note: null,
    archived: false,
    createdAt: '2024-01-15T10:00:00Z',
  },
  {
    id: 2,
    name: 'Demo Broker GmbH',
    icon: 'chart-line',
    color: 'var(--c4)',
    bic: 'DEMODEFFXXX',
    country: 'DE',
    note: null,
    archived: false,
    createdAt: '2024-03-02T10:00:00Z',
  },
  {
    id: 3,
    name: 'Musterkasse',
    icon: 'piggy-bank',
    color: 'var(--c5)',
    bic: 'MUSTDEFFXXX',
    country: 'DE',
    note: null,
    archived: false,
    createdAt: '2024-06-01T10:00:00Z',
  },
];
