pub mod app_config;
pub mod categorization;
pub mod commands;
pub mod db;
pub mod device;
pub mod import_flow;
pub mod importers;
pub mod model;
pub mod pricing_provider;

use tauri::{Emitter, Manager};

use crate::commands::accounts::{
    account_balance, create_account, get_account, get_accounts, update_account, DbState,
};
use crate::commands::admin::{
    backup_database, change_data_path, check_sync_conflicts, check_target_path, find_data_issues,
    force_acquire_sync_lock, get_data_path_info, reset_path_to_default, resolve_conflict_keep_current,
    resolve_conflict_use_other, restore_database, retry_startup, set_path_and_init, validate_backup,
    wipe_database,
};
use crate::commands::aggregates::{
    account_monthly_cashflow, bucket_monthly_flow, cashflow_breakdown, category_breakdown,
    daily_spending, monthly_cashflow, monthly_spending, net_worth_forecast, net_worth_history,
};
use crate::commands::categories::{
    create_category, delete_category, list_categories, merge_categories, update_category,
};
use crate::commands::bucket_rules::{
    apply_bucket_rules_now, create_bucket_rule, delete_bucket_rule, list_bucket_rules,
    update_bucket_rule,
};
use crate::commands::bucket_allocations::{
    create_bucket_allocation, list_bucket_allocations, move_between_buckets, ready_to_assign,
};
use crate::commands::buckets::{
    bucket_balance, create_bucket, delete_bucket, get_bucket, list_bucket_progress,
    list_buckets, update_bucket,
};
use crate::commands::institutions::{
    create_institution, delete_institution, get_institution, list_institutions,
    list_institutions_with_summary, update_institution,
};
use crate::commands::budgets::{
    clear_budget, investment_flow_for_month, list_budget_overrides, month_overview, set_budget,
    uncategorized_monthly_spent,
};
use crate::commands::export::export_transactions_csv;
use crate::commands::fx::{list_currencies, refresh_currency_rate, update_currency_rate};
use crate::commands::import::{import_flatex_pdfs, import_sparkasse_csv, import_trade_republic_csv};
use crate::commands::portfolio::{
    asset_allocation, bucket_holdings, cost_basis_history, cost_basis_history_daily,
    dividend_history, list_holdings, list_security_allocations, portfolio_kpis,
    portfolio_value_by_account_today, realized_gains_summary, set_security_allocations,
};
use crate::commands::prices::{
    fetch_security_history, get_price_history, refresh_prices, set_manual_price, ProviderState,
};
use crate::commands::recurring::{
    create_recurring, delete_recurring, detect_recurring, get_recurring,
    list_recurring, recurring_overview, update_recurring,
};
use crate::commands::rules::{
    apply_rule_to_existing, create_rule, delete_rule, list_rules, preview_rule_match,
    update_rule,
};
use crate::commands::securities::{
    create_security, delete_security, get_security, list_securities, update_security,
};
use crate::commands::breakdowns::{get_breakdown, set_breakdown};
use crate::commands::trades::{
    create_trade, delete_trade, get_trade, list_trades, update_trade,
};
use crate::commands::transactions::{
    aggregate_transactions, assign_account, assign_bucket, assign_category, cleanup_phantom_mirrors,
    create_transaction, delete_transaction, detect_transfers, list_transactions, suggest_category,
    update_transaction,
};
use crate::db::lock::AcquireOutcome;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_accounts,
            create_account,
            get_account,
            update_account,
            account_balance,
            import_trade_republic_csv,
            import_flatex_pdfs,
            import_sparkasse_csv,
            list_transactions,
            aggregate_transactions,
            create_transaction,
            update_transaction,
            delete_transaction,
            suggest_category,
            assign_category,
            list_categories,
            create_category,
            update_category,
            delete_category,
            merge_categories,
            list_rules,
            create_rule,
            update_rule,
            delete_rule,
            apply_rule_to_existing,
            preview_rule_match,
            monthly_spending,
            monthly_cashflow,
            category_breakdown,
            daily_spending,
            account_monthly_cashflow,
            bucket_monthly_flow,
            cashflow_breakdown,
            net_worth_history,
            net_worth_forecast,
            export_transactions_csv,
            list_buckets,
            get_bucket,
            create_bucket,
            update_bucket,
            delete_bucket,
            bucket_balance,
            list_bucket_progress,
            ready_to_assign,
            list_bucket_allocations,
            create_bucket_allocation,
            move_between_buckets,
            list_institutions,
            list_institutions_with_summary,
            get_institution,
            create_institution,
            update_institution,
            delete_institution,
            assign_bucket,
            assign_account,
            list_bucket_rules,
            create_bucket_rule,
            update_bucket_rule,
            delete_bucket_rule,
            apply_bucket_rules_now,
            list_securities,
            get_security,
            create_security,
            update_security,
            delete_security,
            get_breakdown,
            set_breakdown,
            list_trades,
            get_trade,
            create_trade,
            update_trade,
            delete_trade,
            set_budget,
            clear_budget,
            list_budget_overrides,
            month_overview,
            uncategorized_monthly_spent,
            investment_flow_for_month,
            list_holdings,
            realized_gains_summary,
            asset_allocation,
            dividend_history,
            cost_basis_history,
            cost_basis_history_daily,
            portfolio_kpis,
            portfolio_value_by_account_today,
            list_security_allocations,
            set_security_allocations,
            bucket_holdings,
            refresh_prices,
            set_manual_price,
            get_price_history,
            fetch_security_history,
            list_recurring,
            get_recurring,
            create_recurring,
            update_recurring,
            delete_recurring,
            recurring_overview,
            detect_recurring,
            detect_transfers,
            cleanup_phantom_mirrors,
            find_data_issues,
            force_acquire_sync_lock,
            get_data_path_info,
            check_target_path,
            change_data_path,
            backup_database,
            validate_backup,
            restore_database,
            wipe_database,
            retry_startup,
            list_currencies,
            update_currency_rate,
            refresh_currency_rate,
            set_path_and_init,
            reset_path_to_default,
            check_sync_conflicts,
            resolve_conflict_keep_current,
            resolve_conflict_use_other,
        ])
        .setup(|app| {
            let data_dir = {
                #[cfg(target_os = "android")]
                {
                    // Android: external files dir so Syncthing can pick the folder.
                    // Path: /sdcard/Android/data/me.zollner.notchkeep/files/
                    app.path().app_data_dir().expect("app_data_dir")
                }
                #[cfg(not(target_os = "android"))]
                {
                    // Desktop: local data directory (e.g. ~/.local/share/me.zollner.notchkeep/)
                    app.path().app_local_data_dir().expect("app_local_data_dir")
                }
            };
            std::fs::create_dir_all(&data_dir)?;

            let device_id = device::load_or_create(&data_dir)?;
            let hostname = device::hostname();

            let app_cfg = crate::app_config::AppConfig::load(&data_dir)
                .unwrap_or_else(|_| crate::app_config::AppConfig::default(&data_dir));
            let db_path = std::path::PathBuf::from(&app_cfg.db_path);
            let parent_ok = db_path.parent().map(|p| p.is_dir()).unwrap_or(false);

            let pool = if parent_ok {
                tauri::async_runtime::block_on(async {
                    db::connect_file(&db_path).await
                })?
            } else {
                let default_path = crate::app_config::AppConfig::default_db_path(&data_dir);
                let default_pool = tauri::async_runtime::block_on(async {
                    db::connect_file(&default_path).await
                })?;
                let handle = app.handle().clone();
                let bad_path = app_cfg.db_path.clone();
                handle.emit("data_path_error", serde_json::json!({
                    "path": bad_path,
                    "reason": "parent_missing",
                })).ok();
                default_pool
            };

            let outcome = tauri::async_runtime::block_on(async {
                db::lock::acquire(&pool, &device_id, &hostname).await
            })?;
            if let AcquireOutcome::HeldByOther(holder) = &outcome {
                // MVP: log only; UI warning will come with the frontend hook.
                eprintln!(
                    "[notchkeep] sync_lock held by '{}' ({}), acquired {}",
                    holder.hostname, holder.device_id, holder.acquired_at
                );
            }

            let pool_for_bg = pool.clone();
            app.manage(DbState(std::sync::RwLock::new(pool)));
            app.manage(crate::commands::admin::DeviceInfo {
                device_id: device_id.clone(),
                hostname: hostname.clone(),
            });

            // 6e: ProviderState + background price refresh.
            use crate::pricing_provider::yahoo::YahooProvider;
            app.manage(ProviderState(Box::new(YahooProvider::new())));

            let handle_for_bg = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let provider = YahooProvider::new();
                handle_for_bg
                    .emit("price_refresh_status", serde_json::json!({ "stage": "started" }))
                    .ok();
                match crate::db::portfolio::refresh_all_prices(&pool_for_bg, &provider).await {
                    Ok(report) => {
                        handle_for_bg
                            .emit("price_refresh_status", serde_json::json!({
                                "stage": "completed",
                                "report": report,
                            }))
                            .ok();
                    }
                    Err(e) => {
                        eprintln!("[notchkeep] background price refresh failed: {e}");
                        handle_for_bg
                            .emit("price_refresh_status", serde_json::json!({
                                "stage": "failed",
                                "error": e.to_string(),
                            }))
                            .ok();
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            if let (Some(state), Some(dev)) = (
                handle.try_state::<DbState>(),
                handle.try_state::<crate::commands::admin::DeviceInfo>(),
            ) {
                let pool = state.pool();
                let device_id = dev.device_id.clone();
                let _ = tauri::async_runtime::block_on(async move {
                    db::lock::release(&pool, &device_id).await
                });
            }
        }
    });
}

