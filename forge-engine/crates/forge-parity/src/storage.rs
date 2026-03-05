//! SQLite persistence for continuous parity runs.
//!
//! Stores each game result with deck pair, seed, status, divergence details,
//! duration, and covered cards. Provides queries for stats, trends, and heatmaps.

use rusqlite::{params, Connection, Result as SqlResult};

use crate::protocol::{
    ContinuousStats, DeckPairStats, MatchupResult, MatchupStatus, RunRecord, TrendPoint,
};

/// SQLite-backed storage for parity run records.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open (or create) a SQLite database at the given path.
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Open an in-memory database (for tests).
    #[cfg(test)]
    pub fn open_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    fn init_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS runs (
                id                    INTEGER PRIMARY KEY AUTOINCREMENT,
                batch_id              INTEGER NOT NULL,
                deck1                 TEXT NOT NULL,
                deck2                 TEXT NOT NULL,
                seed                  INTEGER NOT NULL,
                status                TEXT NOT NULL,
                snapshots_compared    INTEGER NOT NULL,
                divergence_count      INTEGER NOT NULL,
                first_divergence_field TEXT,
                first_divergence_rust  TEXT,
                first_divergence_java  TEXT,
                covered_cards         TEXT NOT NULL DEFAULT '[]',
                duration_ms           INTEGER NOT NULL,
                error_message         TEXT,
                timestamp             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_runs_timestamp ON runs(timestamp);
            CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
            CREATE INDEX IF NOT EXISTS idx_runs_batch ON runs(batch_id);
            CREATE INDEX IF NOT EXISTS idx_runs_deck_pair ON runs(deck1, deck2);
            ",
        )?;
        Ok(())
    }

    /// Insert a run result into the database.
    pub fn insert_run(
        &self,
        batch_id: i64,
        result: &MatchupResult,
        duration_ms: u64,
    ) -> SqlResult<i64> {
        let status_str = match result.status {
            MatchupStatus::Pass => "pass",
            MatchupStatus::Fail => "fail",
            MatchupStatus::Error => "error",
        };

        let (div_field, div_rust, div_java) = match &result.first_divergence {
            Some(d) => (
                Some(d.field.clone()),
                Some(d.rust_value.clone()),
                Some(d.java_value.clone()),
            ),
            None => (None, None, None),
        };

        let covered_json = serde_json::to_string(&result.covered_cards).unwrap_or_default();

        self.conn.execute(
            "INSERT INTO runs (batch_id, deck1, deck2, seed, status, snapshots_compared,
             divergence_count, first_divergence_field, first_divergence_rust,
             first_divergence_java, covered_cards, duration_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                batch_id,
                result.deck1,
                result.deck2,
                result.seed as i64,
                status_str,
                result.snapshots_compared as i64,
                result.divergence_count as i64,
                div_field,
                div_rust,
                div_java,
                covered_json,
                duration_ms as i64,
                result.error_message,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get aggregate stats across all runs.
    ///
    /// `start_time_iso` filters `games_per_minute` to only count games from
    /// the current session, while totals/pass_rate reflect all historical data.
    pub fn stats(&self, uptime_seconds: u64, start_time_iso: &str) -> SqlResult<ContinuousStats> {
        let total: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0))?;
        let passed: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE status = 'pass'",
            [],
            |r| r.get(0),
        )?;
        let failed: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE status = 'fail'",
            [],
            |r| r.get(0),
        )?;
        let errors: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE status = 'error'",
            [],
            |r| r.get(0),
        )?;

        let pass_rate = if passed + failed > 0 {
            passed as f64 / (passed + failed) as f64
        } else {
            0.0
        };

        // Only count games from the current session for games/minute
        let session_games: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE timestamp >= ?1",
            params![start_time_iso],
            |r| r.get(0),
        )?;

        let games_per_minute = if uptime_seconds > 0 {
            session_games as f64 / (uptime_seconds as f64 / 60.0)
        } else {
            0.0
        };

        let current_batch: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(batch_id), 0) FROM runs",
            [],
            |r| r.get(0),
        )?;

        Ok(ContinuousStats {
            total_games: total,
            passed,
            failed,
            errors,
            pass_rate,
            games_per_minute,
            uptime_seconds,
            current_batch,
        })
    }

    /// Get time-series trend data bucketed by hour or day.
    pub fn trend(&self, bucket: &str, limit: usize) -> SqlResult<Vec<TrendPoint>> {
        let format_str = match bucket {
            "day" => "%Y-%m-%d",
            _ => "%Y-%m-%dT%H:00:00Z", // hour
        };

        let sql = format!(
            "SELECT
                strftime('{format_str}', timestamp) as bucket,
                COUNT(*) as total,
                SUM(CASE WHEN status = 'pass' THEN 1 ELSE 0 END) as passed,
                SUM(CASE WHEN status = 'fail' THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors
             FROM runs
             GROUP BY bucket
             ORDER BY bucket DESC
             LIMIT ?1"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let total: usize = row.get(1)?;
            let passed: usize = row.get(2)?;
            let failed: usize = row.get(3)?;
            let errors: usize = row.get(4)?;
            let pass_rate = if passed + failed > 0 {
                passed as f64 / (passed + failed) as f64
            } else {
                0.0
            };
            Ok(TrendPoint {
                bucket: row.get(0)?,
                total,
                passed,
                failed,
                errors,
                pass_rate,
            })
        })?;

        rows.collect()
    }

    /// Get recent failures with divergence details.
    pub fn recent_failures(&self, limit: usize) -> SqlResult<Vec<RunRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, batch_id, deck1, deck2, seed, status, snapshots_compared,
                    divergence_count, first_divergence_field, first_divergence_rust,
                    first_divergence_java, covered_cards, duration_ms, error_message, timestamp
             FROM runs
             WHERE status IN ('fail', 'error')
             ORDER BY id DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| Self::row_to_record(row))?;
        rows.collect()
    }

    /// Get a specific run by ID.
    pub fn get_run(&self, id: i64) -> SqlResult<RunRecord> {
        self.conn.query_row(
            "SELECT id, batch_id, deck1, deck2, seed, status, snapshots_compared,
                    divergence_count, first_divergence_field, first_divergence_rust,
                    first_divergence_java, covered_cards, duration_ms, error_message, timestamp
             FROM runs WHERE id = ?1",
            params![id],
            |row| Self::row_to_record(row),
        )
    }

    /// Get pass rate heatmap by deck pair.
    pub fn deck_pair_matrix(&self) -> SqlResult<Vec<DeckPairStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT deck1, deck2, COUNT(*) as total,
                    SUM(CASE WHEN status = 'pass' THEN 1 ELSE 0 END) as passed
             FROM runs
             WHERE status != 'error'
             GROUP BY deck1, deck2
             ORDER BY deck1, deck2",
        )?;

        let rows = stmt.query_map([], |row| {
            let total: usize = row.get(2)?;
            let passed: usize = row.get(3)?;
            let pass_rate = if total > 0 {
                passed as f64 / total as f64
            } else {
                0.0
            };
            Ok(DeckPairStats {
                deck1: row.get(0)?,
                deck2: row.get(1)?,
                total,
                passed,
                pass_rate,
            })
        })?;

        rows.collect()
    }

    /// Compute the current pass rate (excluding errors).
    pub fn pass_rate(&self) -> SqlResult<f64> {
        let passed: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE status = 'pass'",
            [],
            |r| r.get(0),
        )?;
        let failed: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM runs WHERE status = 'fail'",
            [],
            |r| r.get(0),
        )?;
        if passed + failed == 0 {
            return Ok(0.0);
        }
        Ok(passed as f64 / (passed + failed) as f64)
    }

    fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRecord> {
        let status_str: String = row.get(5)?;
        let status = match status_str.as_str() {
            "pass" => MatchupStatus::Pass,
            "fail" => MatchupStatus::Fail,
            _ => MatchupStatus::Error,
        };
        let covered_json: String = row.get(11)?;
        let covered_cards: Vec<String> =
            serde_json::from_str(&covered_json).unwrap_or_default();
        let seed_i64: i64 = row.get(4)?;

        Ok(RunRecord {
            id: row.get(0)?,
            batch_id: row.get(1)?,
            deck1: row.get(2)?,
            deck2: row.get(3)?,
            seed: seed_i64 as u64,
            status,
            snapshots_compared: row.get::<_, i64>(6)? as usize,
            divergence_count: row.get::<_, i64>(7)? as usize,
            first_divergence_field: row.get(8)?,
            first_divergence_rust: row.get(9)?,
            first_divergence_java: row.get(10)?,
            covered_cards,
            duration_ms: row.get::<_, i64>(12)? as u64,
            error_message: row.get(13)?,
            timestamp: row.get(14)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Divergence, MatchupResult, MatchupStatus};

    fn make_result(status: MatchupStatus) -> MatchupResult {
        let is_fail = matches!(status, MatchupStatus::Fail);
        let is_error = matches!(status, MatchupStatus::Error);
        MatchupResult {
            deck1: "red_burn".into(),
            deck2: "green_stompy".into(),
            seed: 42,
            status,
            snapshots_compared: 10,
            divergence_count: if is_fail {
                1
            } else {
                0
            },
            first_divergence: if is_fail {
                Some(Divergence {
                    snapshot_index: 3,
                    turn: 2,
                    phase: "Main1".into(),
                    field: "players[0].life".into(),
                    rust_value: "18".into(),
                    java_value: "20".into(),
                })
            } else {
                None
            },
            error_message: if is_error {
                Some("test error".into())
            } else {
                None
            },
            trace: None,
            java_trace: None,
            covered_cards: vec!["Lightning Bolt".into()],
            mechanic_signals: vec![],
            finished_turn: None,
        }
    }

    #[test]
    fn insert_and_query_stats() {
        let db = Storage::open_memory().unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Pass), 100)
            .unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Pass), 150)
            .unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Fail), 200)
            .unwrap();

        let stats = db.stats(60, "2024-01-01T00:00:00Z").unwrap();
        assert_eq!(stats.total_games, 3);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.failed, 1);
        assert!((stats.pass_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn recent_failures_query() {
        let db = Storage::open_memory().unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Pass), 100)
            .unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Fail), 200)
            .unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Error), 50)
            .unwrap();

        let failures = db.recent_failures(10).unwrap();
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].status, MatchupStatus::Error);
        assert_eq!(failures[1].status, MatchupStatus::Fail);
    }

    #[test]
    fn get_run_by_id() {
        let db = Storage::open_memory().unwrap();
        let id = db
            .insert_run(1, &make_result(MatchupStatus::Pass), 100)
            .unwrap();

        let record = db.get_run(id).unwrap();
        assert_eq!(record.deck1, "red_burn");
        assert_eq!(record.seed, 42);
    }

    #[test]
    fn deck_pair_matrix_query() {
        let db = Storage::open_memory().unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Pass), 100)
            .unwrap();
        db.insert_run(1, &make_result(MatchupStatus::Fail), 200)
            .unwrap();

        let matrix = db.deck_pair_matrix().unwrap();
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].total, 2);
        assert_eq!(matrix[0].passed, 1);
    }

    #[test]
    fn pass_rate_empty() {
        let db = Storage::open_memory().unwrap();
        assert_eq!(db.pass_rate().unwrap(), 0.0);
    }
}
