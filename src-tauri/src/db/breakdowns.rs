use serde::Deserialize;
use sqlx::SqlitePool;

use super::{DbError, DbResult};
use crate::model::SecurityBreakdown;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownRowInput {
    pub key: String,
    pub weight_bps: i64,
}

/// Tolerance in bps for sum validation. 50 bps = 0.50%.
pub const SUM_TOLERANCE_BPS: i64 = 50;

/// Pure helper: prüft, ob die Summe der weights im akzeptablen Bereich [10000-tol, 10000+tol] liegt.
/// Returns Ok(()) bei 0 rows (Spezialfall: Breakdown leeren ist erlaubt).
pub fn validate_sum(rows: &[BreakdownRowInput]) -> Result<(), DbError> {
    if rows.is_empty() {
        return Ok(());
    }
    let sum: i64 = rows.iter().map(|r| r.weight_bps).sum();
    if !((10_000 - SUM_TOLERANCE_BPS)..=(10_000 + SUM_TOLERANCE_BPS)).contains(&sum) {
        return Err(DbError::Decode(format!(
            "sum of weights must be 100% ±{:.2}% (got {:.2}%)",
            SUM_TOLERANCE_BPS as f64 / 100.0,
            sum as f64 / 100.0,
        )));
    }
    for r in rows {
        if r.key.trim().is_empty() {
            return Err(DbError::Decode("breakdown key must not be empty".into()));
        }
        if !(0..=10_000).contains(&r.weight_bps) {
            return Err(DbError::Decode(format!(
                "weight_bps out of range [0, 10000]: {}", r.weight_bps,
            )));
        }
    }
    Ok(())
}

pub async fn set_breakdown(
    pool: &SqlitePool,
    security_id: i64,
    dimension: &str,
    rows: &[BreakdownRowInput],
) -> DbResult<()> {
    if !matches!(dimension, "country" | "sector") {
        return Err(DbError::Decode(format!(
            "dimension must be 'country' or 'sector', got {dimension:?}",
        )));
    }
    validate_sum(rows)?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM security_breakdowns WHERE security_id = ?1 AND dimension = ?2")
        .bind(security_id)
        .bind(dimension)
        .execute(&mut *tx)
        .await?;
    for row in rows {
        sqlx::query(
            "INSERT INTO security_breakdowns (security_id, dimension, key, weight_bps)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(security_id)
        .bind(dimension)
        .bind(&row.key)
        .bind(row.weight_bps)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_breakdown(
    pool: &SqlitePool,
    security_id: i64,
    dimension: &str,
) -> DbResult<Vec<SecurityBreakdown>> {
    Ok(sqlx::query_as::<_, SecurityBreakdown>(
        "SELECT security_id, dimension, key, weight_bps
           FROM security_breakdowns
          WHERE security_id = ?1 AND dimension = ?2
          ORDER BY weight_bps DESC, key COLLATE NOCASE ASC",
    )
    .bind(security_id)
    .bind(dimension)
    .fetch_all(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::db::securities::{create_security, NewSecurityPayload};

    #[test]
    fn validate_sum_accepts_exactly_100_percent() {
        let rows = vec![
            BreakdownRowInput { key: "US".into(), weight_bps: 7000 },
            BreakdownRowInput { key: "EU".into(), weight_bps: 3000 },
        ];
        assert!(validate_sum(&rows).is_ok());
    }

    #[test]
    fn validate_sum_accepts_within_tolerance() {
        let rows = vec![
            BreakdownRowInput { key: "US".into(), weight_bps: 6975 },
            BreakdownRowInput { key: "EU".into(), weight_bps: 3000 },
        ]; // sum = 9975 → 99.75%
        assert!(validate_sum(&rows).is_ok());
    }

    #[test]
    fn validate_sum_rejects_outside_tolerance() {
        let rows = vec![
            BreakdownRowInput { key: "US".into(), weight_bps: 5000 },
            BreakdownRowInput { key: "EU".into(), weight_bps: 3000 },
        ]; // 80%
        assert!(validate_sum(&rows).is_err());
    }

    #[test]
    fn validate_sum_accepts_empty_rows() {
        assert!(validate_sum(&[]).is_ok());
    }

    #[test]
    fn validate_sum_rejects_negative_or_oversized_weight() {
        assert!(validate_sum(&[
            BreakdownRowInput { key: "X".into(), weight_bps: -1 },
            BreakdownRowInput { key: "Y".into(), weight_bps: 10001 },
        ]).is_err());
    }

    #[test]
    fn validate_sum_rejects_empty_key() {
        assert!(validate_sum(&[
            BreakdownRowInput { key: "".into(), weight_bps: 10000 },
        ]).is_err());
    }

    #[tokio::test]
    async fn set_breakdown_replaces_existing_rows_for_dimension() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(&pool, NewSecurityPayload {
            isin: "IE00BK5BQT80".into(), symbol: None, name: "x".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();

        let v1 = vec![
            BreakdownRowInput { key: "US".into(), weight_bps: 7000 },
            BreakdownRowInput { key: "EU".into(), weight_bps: 3000 },
        ];
        set_breakdown(&pool, s.id, "country", &v1).await.unwrap();

        let v2 = vec![
            BreakdownRowInput { key: "US".into(), weight_bps: 6000 },
            BreakdownRowInput { key: "JP".into(), weight_bps: 1000 },
            BreakdownRowInput { key: "EU".into(), weight_bps: 3000 },
        ];
        set_breakdown(&pool, s.id, "country", &v2).await.unwrap();

        let stored = get_breakdown(&pool, s.id, "country").await.unwrap();
        assert_eq!(stored.len(), 3);
        let keys: Vec<&str> = stored.iter().map(|r| r.key.as_str()).collect();
        assert!(keys.contains(&"JP"));
        let us = stored.iter().find(|r| r.key == "US").unwrap();
        assert_eq!(us.weight_bps, 6000);
    }

    #[tokio::test]
    async fn set_breakdown_with_empty_rows_clears_existing() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(&pool, NewSecurityPayload {
            isin: "IE00BK5BQT80".into(), symbol: None, name: "x".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();

        set_breakdown(&pool, s.id, "country", &[
            BreakdownRowInput { key: "US".into(), weight_bps: 10000 },
        ]).await.unwrap();
        assert_eq!(get_breakdown(&pool, s.id, "country").await.unwrap().len(), 1);

        set_breakdown(&pool, s.id, "country", &[]).await.unwrap();
        assert!(get_breakdown(&pool, s.id, "country").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_breakdown_rejects_unknown_dimension() {
        let pool = connect_memory().await.unwrap();
        let s = create_security(&pool, NewSecurityPayload {
            isin: "IE00BK5BQT80".into(), symbol: None, name: "x".into(),
            currency: None, asset_type: "etf_equity".into(),
            country: None, sector: None, note: None,
        }).await.unwrap();
        let rows = vec![BreakdownRowInput { key: "US".into(), weight_bps: 10000 }];
        assert!(set_breakdown(&pool, s.id, "industry", &rows).await.is_err());
    }
}
