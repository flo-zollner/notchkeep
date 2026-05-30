export type Lang = 'de' | 'en';

export interface NavStrings {
  overview: string;
  networth: string;
  portfolio: string;
  buckets: string;
  recurring: string;
  transactions: string;
  budgets: string;
  cashflow: string;
  reports: string;
  accounts: string;
  institutions: string;
  settings: string;
}

export interface I18N {
  appName: string;
  nav: NavStrings;
  sections: { main: string; manage: string };
  common: {
    thisMonth: string; lastMonth: string; year: string;
    all: string; month: string; week: string; day: string;
    search: string; add: string; addTx: string; filter: string;
    cancel: string; close: string; save: string; delete: string;
    vs: string; from: string; to: string; compare: string;
    remaining: string; spent: string; ofBudget: string;
    forecast: string; actual: string; planned: string;
    categories: string; account: string; amount: string; date: string;
    name: string; description: string; descriptionPh: string;
    uncategorized: string; income: string; expenses: string; net: string;
    savingsRate: string; netWorth: string; assets: string; liabilities: string;
    liabilitiesSub: string; liabilitiesTooltip: string;
    runway: string; months: string;
    runwaySub: (months: number) => string;
    runwayWindowLabel: string;
    securities: string;
    breakdownMode: string;
    breakdownModeLumped: string;
    breakdownModePerHolding: string;
    breakdownModePerAccount: string;
    breakdownModePerInstitution: string;
    breakdownModePerKind: string;
    newTx: string;
    newTxCash: string;
    newTxTrade: string;
    tradeTitleConnector: string;
    tradeFusionOut: string;
    tradeFusionIn: string;
    categoryHeatmap: string; forecastBand: string;
    connectBank: string; addAccount: string;
    active: string; synced: string;
    back: string; detail: string; balance: string; type: string;
    iban: string; lastSync: string; counterpartyIban: string;
    preferences: string; theme: string; themeAuto: string; themeLight: string; themeDark: string;
    language: string; currency: string;
    privacy: string; hideAmounts: string;
    showCents: string; showCentsDesc: string;
    sync: string; autoSync: string; autoSyncDesc: string;
    categoriesC: string; manageCats: string;
    export: string; exportDesc: string;
    exportTitle: string;
    exportDateFrom: string; exportDateTo: string;
    exportAccountAll: string; exportCategoryAll: string;
    exportSearch: string; exportButton: string;
    exportSuccess: (rows: number) => string;
    exportInvalidRange: string;
    version: string;
    seeAll: string; breakdown: string; perCat: string; monthAvg: string;
    incomeVsExp: string; topCats: string; recent: string;
    yourMoney: string;
    inserted: string; skipped: string; parsed: string;
    rules: string; soon: string;
    edit: string; income_in: string; expense_out: string;
    suggestion: string; applySuggestion: string;
    confirmDelete: string;
    editTx: string;
    catModalTitle: string; newCategory: string;
    icon: string; color: string; budget: string; parent: string;
    topLevel: string; noBudget: string;
    catCount: (n: number) => string;
    rulesC: string; manageRules: string; rulesModalTitle: string;
    newRule: string; noRules: string;
    rulesCount: (n: number) => string;
    matchAll: string; matchAny: string;
    condition: string; addCondition: string;
    field: string; operator: string; value: string;
    fieldCounterparty: string; fieldDescription: string; fieldAmount: string; fieldAccount: string;
    opContains: string; opEquals: string; opStartsWith: string; opEndsWith: string;
    opRegex: string; opRange: string;
    priority: string; enabled: string; targetCategory: string;
    matchPreview: string;
    matchCount: (n: number) => string;
    applyToExisting: string; applyToExistingDesc: string;
    appliedToExisting: (n: number) => string;
    rangeMin: string; rangeMax: string;
    pickAccount: string; pickCategory: string;
    parentAccount: string; topLevelDash: string; invalidIban: string;
    bucket: string; bucketAll: string; bucketNone: string;
    bucketCurrent: string; bucketTarget: string; bucketRemaining: string; bucketReached: string;
    kindBank: string; kindBroker: string; kindSavings: string;
    kindCredit: string; kindCash: string; kindLoan: string;
    subAccounts: string; subAccountsHint: string;
    cycleError: string; ibanDuplicate: string;
    txCountLabel: string;
    institution: string; institutionNone: string;
    bic: string; country: string;
    invalidBic: string; invalidCountry: string;
    flatexPdf: string;
    importFlatex: string;
    kest: string;
    withholdingTax: string;
    importStatements: string;
    parser: string;
    parserFlatex: string;
    parserTr: string;
    parserSparkasse: string;
    selectFiles: string;
    selectedFilesCount: (n: number) => string;
    parserAccept: (parser: 'flatex' | 'tr' | 'sparkasse') => string;
    importWarnings: (n: number) => string;
    importDone: string;
    importErrors: string;
    relativeChange: string;
    relativeChangeHint: string;
  };
  buckets: {
    title: string;
    add: string;
    edit: string;
    delete: string;
    confirmDelete: string;
    empty: string;
    emptyCta: string;
    showArchived: string;
    archived: string;
    name: string;
    targetCents: string;
    startDate: string;
    targetDate: string;
    note: string;
    pickIcon: string;
    pickColor: string;
    errNameRequired: string;
    errTargetInvalid: string;
    errDateOrder: string;
    inSecurities: string;
    sectionFunding: string;
    sectionFunded: string;
    statusFunding: string;
    statusFunded: string;
    readyToAssign: string;
    assign: string;
    assignTo: string;
    moveFrom: string;
    moveTo: string;
    unassignedSource: string;
    errAmountRequired: string;
    errSameEndpoint: string;
    amount: string;
    backToUnassigned: string;
    overspentBy: string;
    coverageWarnTitle: string;
    coverageWarnBody: string;
    coverageFix: string;
    assignPreview: string;
    assignWouldGoNegative: string;
    allocationNote: string;
    occurredOn: string;
    cancel: string;
  };
  accounts: {
    groupBy: string;
    groupByInstitution: string;
    groupByFlat: string;
    withoutInstitution: string;
  };
  institutions: {
    title: string;
    add: string;
    edit: string;
    delete: string;
    confirmDelete: string;
    empty: string;
    emptyCta: string;
    showArchived: string;
    archive: string;
    createNew: string;
    duplicateName: string;
    duplicateBic: string;
    hasAccountsError: (n: number) => string;
    accountCount: (n: number) => string;
    firstTx: string;
    icon: string;
    color: string;
    note: string;
  };
  portfolio: {
    title: string;
    empty: string;
    newSecurity: string;
    new: string;
    showArchived: string;
    tabPositions: string;
    tabDividends: string;
    tabSecurities: string;
    kpiMarketValue: string;
    kpiCostBasis: string;
    kpiUnrealized: string;
    kpiRealizedYtd: string;
    performanceTitle: string;
    emptyPositions: string;
    emptyDividends: string;
    shares: string;
    avgCost: string;
    costBasis: string;
    refreshButton: string;
    refreshBusy: string;
    refreshSuccess: string;
    refreshFailed: string;
    lastUpdate: string;
    pricesNotSet: string;
    refreshHint: string;
    // 6f
    tabAllocation: string;
    allocByAssetType: string;
    allocByCountry: string;
    allocBySector: string;
    unknown: string;
    detailBack: string;
    detailEdit: string;
    priceHistoryTitle: string;
    detailTabTrades: string;
    detailTabDividends: string;
    detailTabBreakdown: string;
    noBreakdown: string;
    rangeDay: string;
    rangeWeek: string;
    rangeManual: string;
    rangeFrom: string;
    rangeTo: string;
    fetchHistoryButton: string;
    fetchHistoryBusy: string;
    fetchHistorySuccess: (n: number) => string;
    fetchHistoryFailed: string;
    // 7d: Bucket allocations
    tabBuckets: string;
    bucketAllocateHint: string;
    bucketAllocAddRow: string;
    bucketAllocBucket: string;
    bucketAllocShares: string;
    bucketAllocRemove: string;
    bucketAllocUnallocated: string;
    bucketAllocTotal: string;
    bucketAllocSave: string;
    bucketAllocSaved: string;
    bucketAllocErrOverAllocation: string;
    bucketAllocNoBuckets: string;
  };
  security: {
    edit: string;
    isin: string;
    symbol: string;
    name: string;
    currency: string;
    assetType: string;
    country: string;
    sector: string;
    note: string;
    archived: string;
    delete: string;
    save: string;
    cancel: string;
    confirmDelete: string;
    errNameRequired: string;
    errIsinInvalid: string;
    errAssetTypeInvalid: string;
    types: {
      stock: string;
      etf_equity: string;
      etf_bond: string;
      etf_reit: string;
      bond: string;
      crypto: string;
      other: string;
    };
  };
  breakdown: {
    country: string;
    sector: string;
    addRow: string;
    removeRow: string;
    key: string;
    weight: string;
    sum: string;
    sumOk: string;
    sumOff: string;
    save: string;
    saved: string;
  };
  budgets: {
    clearOverride: string;
    notSet: string;
    fillStatus: string;
    tabMonth: string;
    tabYear: string;
    rolloverHint: string;
    rolloverFrom: string;
    deficit: string;
    year: string;
    sum: string;
    editCell: string;
  };
  recurring: {
    title: string;
    add: string;
    fromHistory: string;
    upcoming: string;
    active: string;
    showArchived: string;
    statusPaid: string;
    statusPending: string;
    modalTitleNew: string;
    modalTitleEdit: string;
    name: string;
    counterparty: string;
    amount: string;
    expense: string;
    income: string;
    frequency: string;
    weekly: string;
    monthly: string;
    quarterly: string;
    yearly: string;
    anchorDate: string;
    note: string;
    detectTitle: string;
    detectEmpty: string;
    detectAccept: string;
    detectSamples: string;
    confirmDelete: string;
    archive: string;
    unarchive: string;
  };
  trade: {
    titleNew: string;
    titleEdit: string;
    side: string;
    sides: {
      buy: string;
      sell: string;
      dividend: string;
      corporate_action: string;
      tax: string;
    };
    shares: string;
    unitPrice: string;
    fee: string;
    tax: string;
    fxRate: string;
    bookingDate: string;
    account: string;
    security: string;
    counterparty: string;
    note: string;
    amountHint: string;
    save: string;
    cancel: string;
    delete: string;
    confirmDelete: string;
    addTrade: string;
    pickSecurity: string;
    pickSecurityHint: string;
    newSecurity: string;
    errSideRequired: string;
    errAccountRequired: string;
    errSecurityRequired: string;
    errUnitPriceRequired: string;
    errSharesNonZero: string;
    errDateInvalid: string;
  };
  txKind: {
    income: string;
    expense: string;
    transfer: string;
    buy: string;
    sell: string;
    dividend: string;
    corporate_action: string;
    tax: string;
    tax_general: string;
    fee: string;
  };
  cashflow: {
    tabTimeseries: string;
    tabComposition: string;
    periodRange: string;
    periodMonth: string;
    sankeyEmpty: string;
    sankeyOther: string;
    sankeyResetExpansion: string;
    income: string;
    expenses: string;
    balance: string;
    cashflowNode: string;
  };
  data: {
    navLink: string;
    title: string;
    currentLocation: string;
    path: string;
    size: string;
    lastModified: string;
    syncLock: string;
    syncLockSelf: string;
    syncLockOther: string;
    syncLockOtherWarning: string;
    changePath: string;
    backupTitle: string;
    backupHint: string;
    backupButton: string;
    backupSuccess: string;
    restoreTitle: string;
    restoreHint: string;
    restoreButton: string;
    restoreConfirmTitle: string;
    restoreConfirmHint: string;
    restoreConfirmType: string;
    restoreSuccess: string;
    resetTitle: string;
    resetHint: string;
    resetButton: string;
    resetConfirmTitle: string;
    resetConfirmHint: string;
    resetConfirmType: string;
    resetSuccess: string;
    pathChangeTitleExisting: string;
    pathChangeTitleEmpty: string;
    pathChangeUseExisting: string;
    pathChangeOverwriteCopy: string;
    pathChangeMove: string;
    pathChangeCopy: string;
    pathChangeStartFresh: string;
    startupErrorTitle: string;
    startupErrorHint: string;
    startupErrorRetry: string;
    startupErrorPick: string;
    startupErrorDefault: string;
  };
  currencies: {
    title: string;
    addBtn: string;
    refreshAllBtn: string;
    code: string;
    rate: string;
    rateHint: string;
    lastUpdate: string;
    source: string;
    sourceManual: string;
    sourceYahoo: string;
    notInUseBadge: string;
    noRateYet: string;
    editTooltip: string;
    refreshTooltip: string;
    addModalTitle: string;
    addModalCodeLabel: string;
    addModalCodeHint: string;
    addModalFetchBtn: string;
    addModalSaveBtn: string;
    errInvalidCode: string;
    errFetch: string;
    errSaveFailed: string;
  };
  months: string[];
  weekdays: string[];
}

export const I18N: Record<Lang, I18N> = {
  de: {
    appName: 'Notchkeep',
    nav: {
      overview: 'Übersicht',
      networth: 'Vermögen',
      portfolio: 'Depot',
      buckets: 'Töpfe',
      recurring: 'Wiederkehrend',
      transactions: 'Transaktionen',
      budgets: 'Budgets',
      cashflow: 'Cashflow',
      reports: 'Reports',
      accounts: 'Konten',
      institutions: 'Institute',
      settings: 'Einstellungen',
    },
    sections: { main: 'Hauptbereich', manage: 'Verwalten' },
    common: {
      thisMonth: 'Dieser Monat', lastMonth: 'Letzter Monat', year: 'Jahr',
      all: 'Alle', month: 'Monat', week: 'Woche', day: 'Tag',
      search: 'Suchen…', add: 'Hinzufügen', addTx: 'Transaktion',
      filter: 'Filter', cancel: 'Abbrechen', close: 'Schließen', save: 'Speichern', delete: 'Löschen',
      vs: 'vs.', from: 'von', to: 'bis', compare: 'Vergleich',
      remaining: 'Verbleibend', spent: 'Ausgegeben', ofBudget: 'des Budgets',
      forecast: 'Prognose', actual: 'Ist', planned: 'Plan',
      categories: 'Kategorien', account: 'Konto', amount: 'Betrag', date: 'Datum',
      name: 'Name', description: 'Beschreibung',
      descriptionPh: 'Optionale Notiz, Buchungstext, Referenz…',
      uncategorized: 'Unkategorisiert',
      income: 'Einnahmen', expenses: 'Ausgaben', net: 'Differenz',
      savingsRate: 'Sparquote', netWorth: 'Nettovermögen',
      assets: 'Vermögenswerte', liabilities: 'Verbindlichkeiten',
      liabilitiesSub: 'Summe negativer Salden',
      liabilitiesTooltip: 'Summe aller Konten mit negativem Saldo (Kreditkarten, Dispo, Darlehen). Konto-Typ wird nicht gefiltert — jedes Konto mit Minus zählt.',
      runway: 'Reichweite', months: 'Monate',
      runwaySub: (n) => `Liquide ÷ Ø-Ausgaben ${n} Mon.`,
      runwayWindowLabel: 'Mittelung',
      securities: 'Wertpapiere',
      breakdownMode: 'Aufteilung',
      breakdownModeLumped: 'Gesamt',
      breakdownModePerHolding: 'Pro Position',
      breakdownModePerAccount: 'Pro Konto',
      breakdownModePerInstitution: 'Per Institut',
      breakdownModePerKind: 'Pro Typ',
      newTx: 'Neue Transaktion',
      newTxCash: 'Einnahme / Ausgabe',
      newTxTrade: 'Wertpapier-Aktion',
      tradeTitleConnector: 'von',
      tradeFusionOut: 'Fusion-Ausbuchung',
      tradeFusionIn: 'Fusion-Einbuchung',
      categoryHeatmap: 'Tägliche Ausgaben', forecastBand: 'Prognose-Korridor',
      connectBank: 'Bank verbinden', addAccount: 'Konto hinzufügen',
      active: 'Aktiv', synced: 'Synchronisiert',
      back: 'Zurück', detail: 'Konto-Detail', balance: 'Saldo', type: 'Typ',
      iban: 'IBAN', lastSync: 'Letzte Sync.', counterpartyIban: 'Gegenkonto-IBAN',
      preferences: 'Einstellungen', theme: 'Erscheinungsbild',
      themeAuto: 'System', themeLight: 'Hell', themeDark: 'Dunkel',
      language: 'Sprache', currency: 'Währung',
      privacy: 'Datenschutz', hideAmounts: 'Beträge ausblenden',
      showCents: 'Cent-Werte anzeigen', showCentsDesc: '2 Nachkommastellen statt voller Euro',
      sync: 'Synchronisation', autoSync: 'Auto-Sync',
      autoSyncDesc: 'Jede Stunde im Hintergrund',
      categoriesC: 'Kategorien & Regeln', manageCats: 'Kategorien verwalten',
      export: 'Daten exportieren', exportDesc: 'CSV-Datei für Steuerberater',
      exportTitle: 'Transaktionen exportieren',
      exportDateFrom: 'Von', exportDateTo: 'Bis',
      exportAccountAll: 'Alle Konten', exportCategoryAll: 'Alle Kategorien',
      exportSearch: 'Suche', exportButton: 'CSV speichern',
      exportSuccess: (n) => n === 1 ? '1 Transaktion exportiert' : `${n} Transaktionen exportiert`,
      exportInvalidRange: 'Von-Datum darf nicht nach Bis-Datum liegen',
      version: 'Version 0.1.0 · Tauri Build',
      seeAll: 'Alle ansehen', breakdown: 'Aufteilung', perCat: 'Pro Kategorie',
      monthAvg: 'Ø Monat',
      incomeVsExp: 'Einnahmen vs. Ausgaben', topCats: 'Top-Kategorien',
      recent: 'Aktuelle Transaktionen',
      yourMoney: 'Dein Geld, klar gesehen.',
      inserted: 'eingefügt', skipped: 'übersprungen', parsed: 'gelesen',
      rules: 'Regeln', soon: 'Bald verfügbar',
      edit: 'Bearbeiten', income_in: 'Einnahme', expense_out: 'Ausgabe',
      suggestion: 'Vorschlag', applySuggestion: 'Übernehmen',
      confirmDelete: 'Wirklich löschen?',
      editTx: 'Transaktion bearbeiten',
      catModalTitle: 'Kategorien verwalten',
      newCategory: 'Neue Kategorie',
      icon: 'Symbol', color: 'Farbe', budget: 'Budget',
      parent: 'Übergeordnet', topLevel: '── Hauptkategorie ──',
      noBudget: 'Kein Budget',
      catCount: (n: number) => n === 1 ? '1 Kategorie' : `${n} Kategorien`,
      rulesC: 'Regeln', manageRules: 'Regeln verwalten',
      rulesModalTitle: 'Regeln verwalten',
      newRule: 'Neue Regel', noRules: 'Noch keine Regeln',
      rulesCount: (n: number) => n === 1 ? '1 Regel' : `${n} Regeln`,
      matchAll: 'Alle erfüllt (UND)', matchAny: 'Mindestens eine (ODER)',
      condition: 'Bedingung', addCondition: 'Bedingung hinzufügen',
      field: 'Feld', operator: 'Operator', value: 'Wert',
      fieldCounterparty: 'Name', fieldDescription: 'Beschreibung',
      fieldAmount: 'Betrag', fieldAccount: 'Konto',
      opContains: 'Enthält', opEquals: 'Exakt',
      opStartsWith: 'Beginnt mit', opEndsWith: 'Endet mit',
      opRegex: 'Regex', opRange: 'Bereich',
      priority: 'Priorität', enabled: 'Aktiv', targetCategory: 'Zielkategorie',
      matchPreview: 'Match-Vorschau',
      matchCount: (n: number) => n === 1 ? 'trifft auf 1 Tx zu' : `trifft auf ${n} Tx zu`,
      applyToExisting: 'Auf alle anwenden',
      applyToExistingDesc: 'Regel auf bestehende Transaktionen rückwirkend anwenden',
      appliedToExisting: (n: number) =>
        n === 1 ? '1 Transaktion aktualisiert' : `${n} Transaktionen aktualisiert`,
      rangeMin: 'Min (€)', rangeMax: 'Max (€)',
      pickAccount: '— Konto wählen —', pickCategory: '— Kategorie wählen —',
      parentAccount: 'Eltern-Konto',
      topLevelDash: '— Top-Level —',
      invalidIban: 'IBAN ungültig (z.B. DE89 3704 0044 0532 0130 00)',
      bucket: 'Topf', bucketAll: 'Alle Töpfe', bucketNone: '— kein Topf —',
      bucketCurrent: 'Bereits', bucketTarget: 'Ziel', bucketRemaining: 'Noch', bucketReached: 'erreicht ✓',
      kindBank: 'Bank', kindBroker: 'Depot', kindSavings: 'Tagesgeld',
      kindCredit: 'Kreditkarte', kindCash: 'Bargeld', kindLoan: 'Schuld / Darlehen',
      subAccounts: 'Subkonten',
      subAccountsHint: 'Subkonten haben eigene Transaktionen.',
      cycleError: 'Ein Konto kann nicht sich selbst oder ein Subkonto als Eltern haben.',
      ibanDuplicate: 'Diese IBAN ist bereits einem anderen Konto zugeordnet.',
      txCountLabel: 'Tx',
      institution: 'Institut',
      institutionNone: '— Kein Institut —',
      bic: 'BIC',
      country: 'Land',
      invalidBic: 'Ungültige BIC (8 oder 11 Zeichen)',
      invalidCountry: 'Ungültiger Ländercode (2 Buchstaben)',
      flatexPdf: 'Flatex-PDF',
      importFlatex: 'Flatex-PDFs importieren',
      kest: 'KESt',
      withholdingTax: 'Quellensteuer',
      importStatements: 'Statements importieren',
      parser: 'Format',
      parserFlatex: 'Flatex (PDF)',
      parserTr: 'Trade Republic (CSV)',
      parserSparkasse: 'Sparkasse CSV (George)',
      selectFiles: 'Dateien auswählen',
      selectedFilesCount: (n) => n === 1 ? '1 Datei ausgewählt' : `${n} Dateien ausgewählt`,
      parserAccept: (p: 'flatex' | 'tr' | 'sparkasse') => p === 'flatex' ? 'PDF-Belege' : 'CSV-Export',
      importWarnings: (n: number) => `${n} ${n === 1 ? 'Warnung' : 'Warnungen'}`,
      importDone: 'Import abgeschlossen',
      importErrors: 'Fehler beim Import',
      relativeChange: 'Relative Entwicklung (indexiert)',
      relativeChangeHint: 'Erster Datenpunkt = 100%',
    },
    buckets: {
      title: 'Töpfe',
      add: 'Topf hinzufügen',
      edit: 'Topf bearbeiten',
      delete: 'Löschen',
      confirmDelete: 'Diesen Topf wirklich löschen? Transaktionen bleiben erhalten.',
      empty: 'Noch keine Töpfe angelegt.',
      emptyCta: 'Ersten Topf anlegen',
      showArchived: 'Archivierte anzeigen',
      archived: 'Archiviert',
      name: 'Name',
      targetCents: 'Zielbetrag (optional)',
      startDate: 'Startdatum',
      targetDate: 'Zieldatum',
      note: 'Notiz',
      pickIcon: 'Icon',
      pickColor: 'Farbe',
      errNameRequired: 'Bitte einen Namen eingeben.',
      errTargetInvalid: 'Zielbetrag darf nicht negativ sein.',
      errDateOrder: 'Zieldatum muss am oder nach dem Startdatum liegen.',
      inSecurities: 'In Wertpapieren',
      sectionFunding: 'In Funding',
      sectionFunded: 'Funded',
      statusFunding: 'Funding',
      statusFunded: 'Funded',
      readyToAssign: 'Unverteilt',
      assign: 'Zuweisen',
      assignTo: 'In Topf legen',
      moveFrom: 'Von',
      moveTo: 'Nach',
      unassignedSource: '— Unverteilt —',
      errAmountRequired: 'Bitte einen positiven Betrag eingeben.',
      errSameEndpoint: 'Von und Nach müssen unterschiedlich sein.',
      amount: 'Betrag',
      backToUnassigned: 'Zurück nach Unverteilt',
      overspentBy: 'überzogen um {amount}',
      coverageWarnTitle: 'Unterdeckung',
      coverageWarnBody: 'Du hast {amount} mehr reserviert, als auf den Konten liegt.',
      coverageFix: 'Deckung herstellen',
      assignPreview: 'Unverteilt: {before} → {after}',
      assignWouldGoNegative: 'Diese Zuweisung würde „Unverteilt" negativ machen.',
      allocationNote: 'Notiz (optional)',
      occurredOn: 'Datum',
      cancel: 'Abbrechen',
    },
    accounts: {
      groupBy: 'Gruppieren nach',
      groupByInstitution: 'Institut',
      groupByFlat: 'Flach',
      withoutInstitution: 'Ohne Institut',
    },
    institutions: {
      title: 'Institute',
      add: 'Institut hinzufügen',
      edit: 'Institut bearbeiten',
      delete: 'Institut löschen',
      confirmDelete: 'Institut wirklich löschen?',
      empty: 'Noch keine Institute',
      emptyCta: 'Erstes Institut anlegen',
      showArchived: 'Archivierte anzeigen',
      archive: 'Archivieren',
      createNew: '+ Neues Institut anlegen…',
      duplicateName: 'Ein Institut mit diesem Namen existiert bereits.',
      duplicateBic: 'Diese BIC ist bereits einem anderen Institut zugeordnet.',
      hasAccountsError: (n) => `${n} Konten sind diesem Institut zugeordnet. Konten erst umziehen oder archivieren.`,
      accountCount: (n) => `${n} ${n === 1 ? 'Konto' : 'Konten'}`,
      firstTx: 'Älteste Transaktion',
      icon: 'Icon',
      color: 'Farbe',
      note: 'Notiz',
    },
    portfolio: {
      title: 'Depot',
      empty: 'Noch keine Wertpapiere angelegt.',
      newSecurity: 'Neues Wertpapier',
      new: 'Hinzufügen',
      showArchived: 'Archivierte anzeigen',
      tabPositions: 'Positionen',
      tabDividends: 'Dividenden',
      tabSecurities: 'Wertpapiere',
      kpiMarketValue: 'Marktwert',
      kpiCostBasis: 'Eingesetzt',
      kpiUnrealized: 'Unrealisiert',
      kpiRealizedYtd: 'Realisiert (YTD)',
      performanceTitle: 'Performance',
      emptyPositions: 'Keine offenen Positionen',
      emptyDividends: 'Keine Dividenden-Einträge',
      shares: 'Stücke',
      avgCost: 'Ø-Kosten',
      costBasis: 'Cost-Basis',
      refreshButton: 'Kurse aktualisieren',
      refreshBusy: 'Lade Kurse …',
      refreshSuccess: '{n} Kurse aktualisiert',
      refreshFailed: 'Kurs-Abruf fehlgeschlagen',
      lastUpdate: 'Stand: {date}',
      pricesNotSet: 'Kein Kurs',
      refreshHint: 'Lädt aktuelle Kurse + FX von Yahoo Finance. Wird beim App-Start automatisch im Hintergrund gemacht.',
      tabAllocation: 'Allocation',
      allocByAssetType: 'Asset-Typ',
      allocByCountry: 'Region',
      allocBySector: 'Branche',
      unknown: 'Unbekannt',
      detailBack: '← Depot',
      detailEdit: 'Bearbeiten',
      priceHistoryTitle: 'Kursverlauf',
      detailTabTrades: 'Trades',
      detailTabDividends: 'Dividenden',
      detailTabBreakdown: 'Breakdown',
      noBreakdown: 'Kein Breakdown gepflegt',
      rangeDay: '1T',
      rangeWeek: '1W',
      rangeManual: 'Manuell',
      rangeFrom: 'Von',
      rangeTo: 'Bis',
      fetchHistoryButton: 'Kursverlauf laden',
      fetchHistoryBusy: 'Lädt…',
      fetchHistorySuccess: (n: number) => n === 1 ? '1 Kursdatum geladen' : `${n} Kursdaten geladen`,
      fetchHistoryFailed: 'Yahoo-Abruf fehlgeschlagen',
      tabBuckets: 'Töpfe',
      bucketAllocateHint: 'Verteile deine Anteile auf Töpfe — frei aufteilbar, jederzeit anpassbar.',
      bucketAllocAddRow: '+ Topf hinzufügen',
      bucketAllocBucket: 'Topf',
      bucketAllocShares: 'Anteile',
      bucketAllocRemove: 'Entfernen',
      bucketAllocUnallocated: 'Unzugeordnet',
      bucketAllocTotal: 'Summe zugeordnet',
      bucketAllocSave: 'Speichern',
      bucketAllocSaved: 'Zuordnung gespeichert',
      bucketAllocErrOverAllocation: 'Summe übersteigt gehaltene Anteile',
      bucketAllocNoBuckets: 'Noch keine Töpfe angelegt. Geh zu /buckets und leg einen an.',
    },
    security: {
      edit: 'Wertpapier bearbeiten',
      isin: 'ISIN',
      symbol: 'Symbol',
      name: 'Name',
      currency: 'Währung',
      assetType: 'Typ',
      country: 'Land',
      sector: 'Branche',
      note: 'Notiz',
      archived: 'Archiviert',
      delete: 'Löschen',
      save: 'Speichern',
      cancel: 'Abbrechen',
      confirmDelete: 'Wirklich löschen?',
      errNameRequired: 'Name erforderlich',
      errIsinInvalid: 'ISIN-Format ungültig (12 Zeichen, AA##…X).',
      errAssetTypeInvalid: 'Ungültiger Typ',
      types: {
        stock: 'Aktie',
        etf_equity: 'ETF (Aktien)',
        etf_bond: 'ETF (Anleihen)',
        etf_reit: 'ETF (REIT)',
        bond: 'Anleihe',
        crypto: 'Krypto',
        other: 'Sonstige',
      },
    },
    breakdown: {
      country: 'Region',
      sector: 'Branche',
      addRow: 'Zeile hinzufügen',
      removeRow: 'Entfernen',
      key: 'Schlüssel',
      weight: 'Anteil %',
      sum: 'Summe',
      sumOk: '✓ 100.00%',
      sumOff: '!= 100% (Toleranz 0.5%)',
      save: 'Speichern',
      saved: 'Gespeichert',
    },
    budgets: {
      clearOverride: 'Override entfernen',
      notSet: 'kein Budget gesetzt',
      fillStatus: 'Übernommen aus {month}',
      tabMonth: 'Monatsansicht',
      tabYear: 'Jahres-Übersicht',
      rolloverHint: 'Rollover aktivieren — Restbeträge wandern in den nächsten Monat',
      rolloverFrom: '{eur} Rollover aus {month}',
      deficit: '{eur} Defizit aus {month}',
      year: 'Jahr',
      sum: 'Σ',
      editCell: 'Klicken zum Bearbeiten',
    },
    recurring: {
      title: 'Wiederkehrend',
      add: '+ Anlegen',
      fromHistory: 'Aus History',
      upcoming: 'Anstehend',
      active: 'Aktive Standing Orders',
      showArchived: 'Archivierte zeigen',
      statusPaid: 'Bezahlt',
      statusPending: 'Pending',
      modalTitleNew: 'Standing Order anlegen',
      modalTitleEdit: 'Standing Order bearbeiten',
      name: 'Name',
      counterparty: 'Gegenpartei',
      amount: 'Betrag',
      expense: 'Ausgabe',
      income: 'Einnahme',
      frequency: 'Häufigkeit',
      weekly: 'wöchentlich',
      monthly: 'monatlich',
      quarterly: 'vierteljährlich',
      yearly: 'jährlich',
      anchorDate: 'Erste Fälligkeit',
      note: 'Notiz',
      detectTitle: 'Vorschläge aus deinen Transaktionen',
      detectEmpty: 'Keine wiederkehrenden Muster gefunden',
      detectAccept: 'Ausgewählte anlegen',
      detectSamples: '{n} Tx',
      confirmDelete: 'Wirklich löschen?',
      archive: 'Archivieren',
      unarchive: 'Reaktivieren',
    },
    trade: {
      titleNew: 'Neuer Trade',
      titleEdit: 'Trade bearbeiten',
      side: 'Aktion',
      sides: {
        buy: 'Kauf',
        sell: 'Verkauf',
        dividend: 'Dividende',
        corporate_action: 'Kapitalmaßnahme',
        tax: 'Steuer',
      },
      shares: 'Stücke',
      unitPrice: 'Stückpreis',
      fee: 'Gebühr €',
      tax: 'Steuer €',
      fxRate: 'FX-Rate',
      bookingDate: 'Datum',
      account: 'Verrechnungskonto',
      security: 'Wertpapier',
      counterparty: 'Gegenpartei',
      note: 'Notiz',
      amountHint: 'Netto-Cash-Effekt (Kauf negativ, Verkauf/Dividende positiv).',
      save: 'Speichern',
      cancel: 'Abbrechen',
      delete: 'Löschen',
      confirmDelete: 'Wirklich löschen?',
      addTrade: 'Trade hinzufügen',
      pickSecurity: 'Wertpapier wählen…',
      pickSecurityHint: 'Tippen zum Suchen oder ISIN eingeben',
      newSecurity: 'Neues Wertpapier anlegen',
      errSideRequired: 'Aktion erforderlich',
      errAccountRequired: 'Konto erforderlich',
      errSecurityRequired: 'Wertpapier erforderlich',
      errUnitPriceRequired: 'Stückpreis erforderlich für Kauf/Verkauf',
      errSharesNonZero: 'Stücke dürfen nicht 0 sein',
      errDateInvalid: 'Datum YYYY-MM-DD',
    },
    txKind: {
      income: 'Einnahme',
      expense: 'Ausgabe',
      transfer: 'Umbuchung',
      buy: 'Kauf',
      sell: 'Verkauf',
      dividend: 'Dividende',
      corporate_action: 'Kapitalmaßnahme',
      tax: 'Steuer',
      tax_general: 'Steuer',
      fee: 'Gebühr',
    },
    cashflow: {
      tabTimeseries: 'Zeitreihe',
      tabComposition: 'Komposition',
      periodRange: 'Zeitraum',
      periodMonth: 'Monat',
      sankeyEmpty: 'Keine Cashflow-Daten im Zeitraum',
      sankeyOther: 'Sonstige',
      sankeyResetExpansion: 'Drilldown zurücksetzen',
      income: 'Einnahmen',
      expenses: 'Ausgaben',
      balance: 'Saldo',
      cashflowNode: 'Cashflow',
    },
    currencies: {
      title: 'Währungen & Wechselkurse',
      addBtn: 'Hinzufügen',
      refreshAllBtn: 'Alle aktualisieren',
      code: 'Code',
      rate: 'Kurs (1 → EUR)',
      rateHint: 'Wieviel EUR pro 1 Einheit Fremdwährung',
      lastUpdate: 'Letzte Aktualisierung',
      source: 'Quelle',
      sourceManual: 'manuell',
      sourceYahoo: 'yahoo',
      notInUseBadge: 'ungenutzt',
      noRateYet: 'noch keine Rate',
      editTooltip: 'Manuell bearbeiten',
      refreshTooltip: 'Aus dem Web aktualisieren',
      addModalTitle: 'Währung hinzufügen',
      addModalCodeLabel: 'Currency-Code',
      addModalCodeHint: '3 Buchstaben (z. B. JPY, GBP)',
      addModalFetchBtn: 'Holen & speichern',
      addModalSaveBtn: 'Speichern',
      errInvalidCode: 'Currency-Code muss aus 3 Buchstaben bestehen',
      errFetch: 'Konnte Kurs nicht abrufen',
      errSaveFailed: 'Speichern fehlgeschlagen',
    },
    data: {
      navLink: 'Datenverwaltung →',
      title: 'Datenverwaltung',
      currentLocation: 'Aktueller Speicherort',
      path: 'Pfad',
      size: 'Größe',
      lastModified: 'Letzte Änderung',
      syncLock: 'Sync-Lock',
      syncLockSelf: 'Dieses Gerät',
      syncLockOther: 'Anderes Gerät',
      syncLockOtherWarning: 'Schreibvorgänge können zu Konflikten führen.',
      changePath: 'Pfad ändern…',
      backupTitle: 'Backup erstellen',
      backupHint: 'Erstellt eine vollständige Kopie der DB als .sqlite-Datei. Snapshot-basiert, kein Risiko für die laufende DB.',
      backupButton: 'Backup erstellen…',
      backupSuccess: 'Backup erstellt',
      restoreTitle: 'Backup einspielen',
      restoreHint: 'Wählt ein Backup-File und ersetzt die aktuelle DB damit. Aktuelle Daten gehen verloren — vorher Backup erstellen!',
      restoreButton: 'Backup einspielen…',
      restoreConfirmTitle: 'Backup wirklich einspielen?',
      restoreConfirmHint: 'Aktuelle Daten werden ersetzt. Aktion ist nicht rückgängig zu machen.',
      restoreConfirmType: 'Tippe „restore" zum Bestätigen',
      restoreSuccess: 'Backup eingespielt',
      resetTitle: 'Alle Daten löschen',
      resetHint: 'Setzt die App auf den Auslieferungszustand zurück. Konten, Transaktionen, Kategorien, Wiederkehrende — alles weg. Vorher Backup erstellen!',
      resetButton: 'Alle Daten löschen…',
      resetConfirmTitle: 'Wirklich alle Daten löschen?',
      resetConfirmHint: 'Alle Daten werden permanent gelöscht. Aktion ist nicht rückgängig zu machen.',
      resetConfirmType: 'Tippe „löschen" zum Bestätigen',
      resetSuccess: 'Daten gelöscht. Du startest mit einer leeren DB.',
      pathChangeTitleExisting: 'Am Zielort liegt bereits eine DB',
      pathChangeTitleEmpty: 'Am Zielort liegt noch keine DB',
      pathChangeUseExisting: 'Vorhandene DB verwenden',
      pathChangeOverwriteCopy: 'Aktuelle dorthin kopieren (überschreibt!)',
      pathChangeMove: 'Aktuelle dorthin verschieben',
      pathChangeCopy: 'Aktuelle dorthin kopieren',
      pathChangeStartFresh: 'Leere DB anlegen',
      startupErrorTitle: 'DB-Pfad nicht erreichbar',
      startupErrorHint: 'Der konfigurierte DB-Pfad ist nicht erreichbar. Mögliche Ursachen: Syncthing-Ordner nicht synchronisiert, externes Laufwerk nicht gemountet.',
      startupErrorRetry: 'Erneut versuchen',
      startupErrorPick: 'Anderen Pfad wählen…',
      startupErrorDefault: 'Zurück zum Standardpfad',
    },
    months: ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'],
    weekdays: ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'],
  },
  en: {
    appName: 'Notchkeep',
    nav: {
      overview: 'Overview',
      networth: 'Net Worth',
      portfolio: 'Portfolio',
      buckets: 'Buckets',
      recurring: 'Recurring',
      transactions: 'Transactions',
      budgets: 'Budgets',
      cashflow: 'Cashflow',
      reports: 'Reports',
      accounts: 'Accounts',
      institutions: 'Institutions',
      settings: 'Settings',
    },
    sections: { main: 'Main', manage: 'Manage' },
    common: {
      thisMonth: 'This month', lastMonth: 'Last month', year: 'Year',
      all: 'All', month: 'Month', week: 'Week', day: 'Day',
      search: 'Search…', add: 'Add', addTx: 'Transaction',
      filter: 'Filter', cancel: 'Cancel', close: 'Close', save: 'Save', delete: 'Delete',
      vs: 'vs.', from: 'from', to: 'to', compare: 'Compare',
      remaining: 'Remaining', spent: 'Spent', ofBudget: 'of budget',
      forecast: 'Forecast', actual: 'Actual', planned: 'Planned',
      categories: 'Categories', account: 'Account', amount: 'Amount', date: 'Date',
      name: 'Name', description: 'Description',
      descriptionPh: 'Optional note, bank memo, reference…',
      uncategorized: 'Uncategorized',
      income: 'Income', expenses: 'Expenses', net: 'Net',
      savingsRate: 'Savings rate', netWorth: 'Net worth',
      assets: 'Assets', liabilities: 'Liabilities',
      liabilitiesSub: 'Sum of negative balances',
      liabilitiesTooltip: 'Sum of all accounts with negative balance (credit cards, overdraft, loans). Account kind is not filtered — every account in the red counts.',
      runway: 'Runway', months: 'months',
      runwaySub: (n) => `Liquid ÷ avg expense ${n}mo`,
      runwayWindowLabel: 'Window',
      securities: 'Securities',
      breakdownMode: 'Breakdown',
      breakdownModeLumped: 'Combined',
      breakdownModePerHolding: 'Per holding',
      breakdownModePerAccount: 'Per account',
      breakdownModePerInstitution: 'Per institution',
      breakdownModePerKind: 'Per type',
      newTx: 'New transaction',
      newTxCash: 'Income / Expense',
      newTxTrade: 'Securities action',
      tradeTitleConnector: 'of',
      tradeFusionOut: 'Merger withdrawal',
      tradeFusionIn: 'Merger deposit',
      categoryHeatmap: 'Daily spending', forecastBand: 'Forecast cone',
      connectBank: 'Connect bank', addAccount: 'Add account',
      active: 'Active', synced: 'Synced',
      back: 'Back', detail: 'Account detail', balance: 'Balance', type: 'Type',
      iban: 'IBAN', lastSync: 'Last sync', counterpartyIban: 'Counterparty IBAN',
      preferences: 'Preferences', theme: 'Appearance',
      themeAuto: 'System', themeLight: 'Light', themeDark: 'Dark',
      language: 'Language', currency: 'Currency',
      privacy: 'Privacy', hideAmounts: 'Hide amounts',
      showCents: 'Show cents', showCentsDesc: '2 decimal places instead of full euros',
      sync: 'Sync', autoSync: 'Auto-sync',
      autoSyncDesc: 'Background every hour',
      categoriesC: 'Categories & rules', manageCats: 'Manage categories',
      export: 'Export data', exportDesc: 'CSV for accountant',
      exportTitle: 'Export transactions',
      exportDateFrom: 'From', exportDateTo: 'To',
      exportAccountAll: 'All accounts', exportCategoryAll: 'All categories',
      exportSearch: 'Search', exportButton: 'Save CSV',
      exportSuccess: (n) => n === 1 ? '1 transaction exported' : `${n} transactions exported`,
      exportInvalidRange: 'From date must not be after To date',
      version: 'Version 0.1.0 · Tauri build',
      seeAll: 'See all', breakdown: 'Breakdown', perCat: 'By category',
      monthAvg: 'Monthly avg',
      incomeVsExp: 'Income vs. expenses', topCats: 'Top categories',
      recent: 'Recent transactions',
      yourMoney: 'Your money, clearly.',
      inserted: 'inserted', skipped: 'skipped', parsed: 'parsed',
      rules: 'Rules', soon: 'Coming soon',
      edit: 'Edit', income_in: 'Income', expense_out: 'Expense',
      suggestion: 'Suggestion', applySuggestion: 'Apply',
      confirmDelete: 'Delete for real?',
      editTx: 'Edit transaction',
      catModalTitle: 'Manage categories',
      newCategory: 'New category',
      icon: 'Icon', color: 'Color', budget: 'Budget',
      parent: 'Parent', topLevel: '── Top level ──',
      noBudget: 'No budget',
      catCount: (n: number) => n === 1 ? '1 category' : `${n} categories`,
      rulesC: 'Rules', manageRules: 'Manage rules',
      rulesModalTitle: 'Manage rules',
      newRule: 'New rule', noRules: 'No rules yet',
      rulesCount: (n: number) => n === 1 ? '1 rule' : `${n} rules`,
      matchAll: 'All match (AND)', matchAny: 'Any match (OR)',
      condition: 'Condition', addCondition: 'Add condition',
      field: 'Field', operator: 'Operator', value: 'Value',
      fieldCounterparty: 'Name', fieldDescription: 'Description',
      fieldAmount: 'Amount', fieldAccount: 'Account',
      opContains: 'Contains', opEquals: 'Equals',
      opStartsWith: 'Starts with', opEndsWith: 'Ends with',
      opRegex: 'Regex', opRange: 'Range',
      priority: 'Priority', enabled: 'Enabled', targetCategory: 'Target category',
      matchPreview: 'Match preview',
      matchCount: (n: number) => n === 1 ? 'matches 1 tx' : `matches ${n} txs`,
      applyToExisting: 'Apply to existing',
      applyToExistingDesc: 'Apply rule retroactively to existing transactions',
      appliedToExisting: (n: number) =>
        n === 1 ? '1 transaction updated' : `${n} transactions updated`,
      rangeMin: 'Min (€)', rangeMax: 'Max (€)',
      pickAccount: '— Pick account —', pickCategory: '— Pick category —',
      parentAccount: 'Parent account',
      topLevelDash: '— Top-level —',
      invalidIban: 'IBAN invalid (e.g. DE89 3704 0044 0532 0130 00)',
      bucket: 'Bucket', bucketAll: 'All buckets', bucketNone: '— no bucket —',
      bucketCurrent: 'Current', bucketTarget: 'Target', bucketRemaining: 'Remaining', bucketReached: 'reached ✓',
      kindBank: 'Bank', kindBroker: 'Brokerage', kindSavings: 'Savings',
      kindCredit: 'Credit card', kindCash: 'Cash', kindLoan: 'Loan',
      subAccounts: 'Sub-accounts',
      subAccountsHint: 'Sub-accounts have their own transactions.',
      cycleError: 'An account cannot have itself or one of its sub-accounts as parent.',
      ibanDuplicate: 'This IBAN is already assigned to another account.',
      txCountLabel: 'tx',
      institution: 'Institution',
      institutionNone: '— No institution —',
      bic: 'BIC',
      country: 'Country',
      invalidBic: 'Invalid BIC (8 or 11 characters)',
      invalidCountry: 'Invalid country code (2 letters)',
      flatexPdf: 'Flatex PDF',
      importFlatex: 'Import Flatex PDFs',
      kest: 'KESt',
      withholdingTax: 'Withholding tax',
      importStatements: 'Import statements',
      parser: 'Format',
      parserFlatex: 'Flatex (PDF)',
      parserTr: 'Trade Republic (CSV)',
      parserSparkasse: 'Sparkasse CSV (George)',
      selectFiles: 'Select files',
      selectedFilesCount: (n) => n === 1 ? '1 file selected' : `${n} files selected`,
      parserAccept: (p: 'flatex' | 'tr' | 'sparkasse') => p === 'flatex' ? 'PDF statements' : 'CSV export',
      importWarnings: (n: number) => `${n} ${n === 1 ? 'warning' : 'warnings'}`,
      importDone: 'Import complete',
      importErrors: 'Import errors',
      relativeChange: 'Relative change (indexed)',
      relativeChangeHint: 'First data point = 100%',
    },
    buckets: {
      title: 'Buckets',
      add: 'Add bucket',
      edit: 'Edit bucket',
      delete: 'Delete',
      confirmDelete: 'Really delete this bucket? Transactions are kept.',
      empty: 'No buckets yet.',
      emptyCta: 'Create first bucket',
      showArchived: 'Show archived',
      archived: 'Archived',
      name: 'Name',
      targetCents: 'Target amount (optional)',
      startDate: 'Start date',
      targetDate: 'Target date',
      note: 'Note',
      pickIcon: 'Icon',
      pickColor: 'Color',
      errNameRequired: 'Please enter a name.',
      errTargetInvalid: 'Target amount must be non-negative.',
      errDateOrder: 'Target date must be on or after start date.',
      inSecurities: 'In securities',
      sectionFunding: 'Funding',
      sectionFunded: 'Funded',
      statusFunding: 'Funding',
      statusFunded: 'Funded',
      readyToAssign: 'Ready to Assign',
      assign: 'Assign',
      assignTo: 'Add to bucket',
      moveFrom: 'From',
      moveTo: 'To',
      unassignedSource: '— Ready to Assign —',
      errAmountRequired: 'Enter a positive amount.',
      errSameEndpoint: 'From and To must differ.',
      amount: 'Amount',
      backToUnassigned: 'Back to Ready to Assign',
      overspentBy: 'overspent by {amount}',
      coverageWarnTitle: 'Under-funded',
      coverageWarnBody: "You've reserved {amount} more than your accounts hold.",
      coverageFix: 'Fix coverage',
      assignPreview: 'Ready to Assign: {before} → {after}',
      assignWouldGoNegative: 'This assignment would make Ready to Assign negative.',
      allocationNote: 'Note (optional)',
      occurredOn: 'Date',
      cancel: 'Cancel',
    },
    accounts: {
      groupBy: 'Group by',
      groupByInstitution: 'Institution',
      groupByFlat: 'Flat',
      withoutInstitution: 'Without institution',
    },
    institutions: {
      title: 'Institutions',
      add: 'Add institution',
      edit: 'Edit institution',
      delete: 'Delete institution',
      confirmDelete: 'Really delete this institution?',
      empty: 'No institutions yet',
      emptyCta: 'Create first institution',
      showArchived: 'Show archived',
      archive: 'Archive',
      createNew: '+ Create new institution…',
      duplicateName: 'An institution with this name already exists.',
      duplicateBic: 'This BIC is already assigned to another institution.',
      hasAccountsError: (n) => `${n} accounts are assigned to this institution. Move or archive them first.`,
      accountCount: (n) => `${n} ${n === 1 ? 'account' : 'accounts'}`,
      firstTx: 'Oldest transaction',
      icon: 'Icon',
      color: 'Color',
      note: 'Note',
    },
    portfolio: {
      title: 'Portfolio',
      empty: 'No securities yet.',
      newSecurity: 'New security',
      new: 'Add',
      showArchived: 'Show archived',
      tabPositions: 'Positions',
      tabDividends: 'Dividends',
      tabSecurities: 'Securities',
      kpiMarketValue: 'Market value',
      kpiCostBasis: 'Cost',
      kpiUnrealized: 'Unrealized',
      kpiRealizedYtd: 'Realized (YTD)',
      performanceTitle: 'Performance',
      emptyPositions: 'No open positions',
      emptyDividends: 'No dividend entries',
      shares: 'Shares',
      avgCost: 'Avg cost',
      costBasis: 'Cost basis',
      refreshButton: 'Refresh prices',
      refreshBusy: 'Loading prices …',
      refreshSuccess: '{n} prices updated',
      refreshFailed: 'Price fetch failed',
      lastUpdate: 'As of {date}',
      pricesNotSet: 'No price',
      refreshHint: 'Loads latest prices + FX from Yahoo Finance. Runs in background on app start.',
      tabAllocation: 'Allocation',
      allocByAssetType: 'Asset type',
      allocByCountry: 'Region',
      allocBySector: 'Sector',
      unknown: 'Unknown',
      detailBack: '← Portfolio',
      detailEdit: 'Edit',
      priceHistoryTitle: 'Price history',
      detailTabTrades: 'Trades',
      detailTabDividends: 'Dividends',
      detailTabBreakdown: 'Breakdown',
      noBreakdown: 'No breakdown defined',
      rangeDay: '1D',
      rangeWeek: '1W',
      rangeManual: 'Custom',
      rangeFrom: 'From',
      rangeTo: 'To',
      fetchHistoryButton: 'Fetch price history',
      fetchHistoryBusy: 'Loading…',
      fetchHistorySuccess: (n: number) => n === 1 ? '1 price point loaded' : `${n} price points loaded`,
      fetchHistoryFailed: 'Yahoo fetch failed',
      tabBuckets: 'Buckets',
      bucketAllocateHint: 'Split your shares across buckets — fully customizable, adjust anytime.',
      bucketAllocAddRow: '+ Add bucket',
      bucketAllocBucket: 'Bucket',
      bucketAllocShares: 'Shares',
      bucketAllocRemove: 'Remove',
      bucketAllocUnallocated: 'Unallocated',
      bucketAllocTotal: 'Total allocated',
      bucketAllocSave: 'Save',
      bucketAllocSaved: 'Allocation saved',
      bucketAllocErrOverAllocation: 'Sum exceeds held shares',
      bucketAllocNoBuckets: 'No buckets yet. Create one at /buckets first.',
    },
    security: {
      edit: 'Edit security',
      isin: 'ISIN',
      symbol: 'Symbol',
      name: 'Name',
      currency: 'Currency',
      assetType: 'Type',
      country: 'Country',
      sector: 'Sector',
      note: 'Note',
      archived: 'Archived',
      delete: 'Delete',
      save: 'Save',
      cancel: 'Cancel',
      confirmDelete: 'Really delete?',
      errNameRequired: 'Name required',
      errIsinInvalid: 'Invalid ISIN format (12 chars, AA##…X).',
      errAssetTypeInvalid: 'Invalid type',
      types: {
        stock: 'Stock',
        etf_equity: 'ETF (Equity)',
        etf_bond: 'ETF (Bond)',
        etf_reit: 'ETF (REIT)',
        bond: 'Bond',
        crypto: 'Crypto',
        other: 'Other',
      },
    },
    breakdown: {
      country: 'Region',
      sector: 'Sector',
      addRow: 'Add row',
      removeRow: 'Remove',
      key: 'Key',
      weight: 'Weight %',
      sum: 'Sum',
      sumOk: '✓ 100.00%',
      sumOff: '!= 100% (tolerance 0.5%)',
      save: 'Save',
      saved: 'Saved',
    },
    budgets: {
      clearOverride: 'Remove override',
      notSet: 'no budget set',
      fillStatus: 'Inherited from {month}',
      tabMonth: 'Monthly view',
      tabYear: 'Yearly overview',
      rolloverHint: 'Enable rollover — leftover amounts carry into the next month',
      rolloverFrom: '{eur} rollover from {month}',
      deficit: '{eur} deficit from {month}',
      year: 'Year',
      sum: 'Σ',
      editCell: 'Click to edit',
    },
    recurring: {
      title: 'Recurring',
      add: '+ Add',
      fromHistory: 'From history',
      upcoming: 'Upcoming',
      active: 'Active standing orders',
      showArchived: 'Show archived',
      statusPaid: 'Paid',
      statusPending: 'Pending',
      modalTitleNew: 'Add standing order',
      modalTitleEdit: 'Edit standing order',
      name: 'Name',
      counterparty: 'Counterparty',
      amount: 'Amount',
      expense: 'Expense',
      income: 'Income',
      frequency: 'Frequency',
      weekly: 'weekly',
      monthly: 'monthly',
      quarterly: 'quarterly',
      yearly: 'yearly',
      anchorDate: 'First due date',
      note: 'Note',
      detectTitle: 'Suggestions from your transactions',
      detectEmpty: 'No recurring patterns found',
      detectAccept: 'Add selected',
      detectSamples: '{n} tx',
      confirmDelete: 'Really delete?',
      archive: 'Archive',
      unarchive: 'Restore',
    },
    trade: {
      titleNew: 'New trade',
      titleEdit: 'Edit trade',
      side: 'Action',
      sides: {
        buy: 'Buy',
        sell: 'Sell',
        dividend: 'Dividend',
        corporate_action: 'Corporate action',
        tax: 'Tax',
      },
      shares: 'Shares',
      unitPrice: 'Unit price',
      fee: 'Fee €',
      tax: 'Tax €',
      fxRate: 'FX rate',
      bookingDate: 'Date',
      account: 'Cash account',
      security: 'Security',
      counterparty: 'Counterparty',
      note: 'Note',
      amountHint: 'Net cash effect (buy negative, sell/dividend positive).',
      save: 'Save',
      cancel: 'Cancel',
      delete: 'Delete',
      confirmDelete: 'Really delete?',
      addTrade: 'Add trade',
      pickSecurity: 'Pick security…',
      pickSecurityHint: 'Type to search or enter ISIN',
      newSecurity: 'New security',
      errSideRequired: 'Action required',
      errAccountRequired: 'Account required',
      errSecurityRequired: 'Security required',
      errUnitPriceRequired: 'Unit price required for buy/sell',
      errSharesNonZero: 'Shares must not be 0',
      errDateInvalid: 'Date YYYY-MM-DD',
    },
    txKind: {
      income: 'Income',
      expense: 'Expense',
      transfer: 'Transfer',
      buy: 'Buy',
      sell: 'Sell',
      dividend: 'Dividend',
      corporate_action: 'Corporate action',
      tax: 'Tax',
      tax_general: 'Tax',
      fee: 'Fee',
    },
    cashflow: {
      tabTimeseries: 'Time series',
      tabComposition: 'Composition',
      periodRange: 'Range',
      periodMonth: 'Month',
      sankeyEmpty: 'No cashflow data in range',
      sankeyOther: 'Other',
      sankeyResetExpansion: 'Reset drilldown',
      income: 'Income',
      expenses: 'Expenses',
      balance: 'Balance',
      cashflowNode: 'Cashflow',
    },
    currencies: {
      title: 'Currencies & Exchange Rates',
      addBtn: 'Add',
      refreshAllBtn: 'Refresh all',
      code: 'Code',
      rate: 'Rate (1 → EUR)',
      rateHint: 'How many EUR per 1 unit of foreign currency',
      lastUpdate: 'Last update',
      source: 'Source',
      sourceManual: 'manual',
      sourceYahoo: 'yahoo',
      notInUseBadge: 'unused',
      noRateYet: 'no rate yet',
      editTooltip: 'Edit manually',
      refreshTooltip: 'Refresh from web',
      addModalTitle: 'Add currency',
      addModalCodeLabel: 'Currency code',
      addModalCodeHint: '3 letters (e.g., JPY, GBP)',
      addModalFetchBtn: 'Fetch & save',
      addModalSaveBtn: 'Save',
      errInvalidCode: 'Currency code must be 3 letters',
      errFetch: 'Could not fetch rate',
      errSaveFailed: 'Save failed',
    },
    data: {
      navLink: 'Data management →',
      title: 'Data management',
      currentLocation: 'Current location',
      path: 'Path',
      size: 'Size',
      lastModified: 'Last modified',
      syncLock: 'Sync lock',
      syncLockSelf: 'This device',
      syncLockOther: 'Another device',
      syncLockOtherWarning: 'Writes may cause conflicts.',
      changePath: 'Change path…',
      backupTitle: 'Create backup',
      backupHint: 'Creates a complete copy of the DB as a .sqlite file. Snapshot-based, no risk to the running DB.',
      backupButton: 'Create backup…',
      backupSuccess: 'Backup created',
      restoreTitle: 'Restore backup',
      restoreHint: 'Picks a backup file and replaces the current DB. Current data will be lost — make a backup first!',
      restoreButton: 'Restore backup…',
      restoreConfirmTitle: 'Restore this backup?',
      restoreConfirmHint: 'Current data will be replaced. This action cannot be undone.',
      restoreConfirmType: 'Type "restore" to confirm',
      restoreSuccess: 'Backup restored',
      resetTitle: 'Delete all data',
      resetHint: 'Resets the app to factory state. Accounts, transactions, categories, recurring — all gone. Make a backup first!',
      resetButton: 'Delete all data…',
      resetConfirmTitle: 'Really delete all data?',
      resetConfirmHint: 'All data will be permanently deleted. This action cannot be undone.',
      resetConfirmType: 'Type "delete" to confirm',
      resetSuccess: 'Data deleted. You\'re starting with an empty DB.',
      pathChangeTitleExisting: 'A DB already exists at the target',
      pathChangeTitleEmpty: 'No DB at the target yet',
      pathChangeUseExisting: 'Use existing DB',
      pathChangeOverwriteCopy: 'Copy current there (overwrites!)',
      pathChangeMove: 'Move current there',
      pathChangeCopy: 'Copy current there',
      pathChangeStartFresh: 'Create empty DB',
      startupErrorTitle: 'DB path unreachable',
      startupErrorHint: 'The configured DB path is unreachable. Possible causes: Syncthing folder not synced, external drive not mounted.',
      startupErrorRetry: 'Retry',
      startupErrorPick: 'Choose another path…',
      startupErrorDefault: 'Back to default path',
    },
    months: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
    weekdays: ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'],
  },
};
