//! Native SQLite adapter. Business rules live only in skarma-core.
use rusqlite::{Connection, TransactionBehavior, params_from_iter, types::Value as NativeValue};
pub use skarma_core::store::{ApiError, Result};
use skarma_core::{
    Command,
    model::*,
    sql::{Sql, Value},
};

struct Sqlite<'a>(&'a Connection);
fn values(params: &[Value]) -> Vec<NativeValue> {
    params
        .iter()
        .map(|value| match value {
            Value::Null => NativeValue::Null,
            Value::Text(text) => NativeValue::Text(text.clone()),
            Value::Integer(n) => NativeValue::Integer(*n),
        })
        .collect()
}
impl Sql for Sqlite<'_> {
    fn execute(&self, sql: &str, params: &[Value]) -> Result<()> {
        self.0
            .execute(sql, params_from_iter(values(params)))
            .map_err(|_| ApiError::storage())?;
        Ok(())
    }
    fn execute_batch(&self, sql: &str) -> Result<()> {
        self.0.execute_batch(sql).map_err(|_| ApiError::storage())
    }
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<String>>> {
        let mut statement = self.0.prepare(sql).map_err(|_| ApiError::storage())?;
        let columns = statement.column_count();
        let rows = statement
            .query_map(params_from_iter(values(params)), |row| {
                (0..columns)
                    .map(|index| row.get::<_, String>(index))
                    .collect()
            })
            .map_err(|_| ApiError::storage())?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(|_| ApiError::storage())
    }
}
fn transaction<T>(db: &mut Connection, command: impl FnOnce(&dyn Sql) -> Result<T>) -> Result<T> {
    // Acquire the write reservation before reading versions, including across processes.
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| ApiError::storage())?;
    let result = command(&Sqlite(&tx))?;
    tx.commit().map_err(|_| ApiError::storage())?;
    Ok(result)
}

pub fn initialize(db: &Connection) -> Result<()> {
    // This setting is per connection and must be applied before entering a transaction.
    db.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|_| ApiError::storage())?;
    let tx = rusqlite::Transaction::new_unchecked(db, TransactionBehavior::Immediate)
        .map_err(|_| ApiError::storage())?;
    skarma_core::store::initialize(&Sqlite(&tx))?;
    tx.commit().map_err(|_| ApiError::storage())
}
pub fn execute(db: &mut Connection, command: Command) -> Result<serde_json::Value> {
    transaction(db, |sql| skarma_core::execute(sql, command))
}
pub fn state(db: &Connection) -> Result<AppState> {
    skarma_core::store::state(&Sqlite(db))
}
pub fn goal_history(db: &Connection, id: &str) -> Result<Vec<GoalRevision>> {
    skarma_core::store::goal_history(&Sqlite(db), id)
}
pub fn create_goal(db: &mut Connection, input: NewGoal) -> Result<Goal> {
    transaction(db, |sql| skarma_core::store::create_goal(sql, input))
}
pub fn update_goal(db: &mut Connection, id: &str, request: UpdateGoal) -> Result<Goal> {
    transaction(db, |sql| skarma_core::store::update_goal(sql, id, request))
}
pub fn save_session(db: &mut Connection, id: &str, request: SaveSession) -> Result<SaveResult> {
    transaction(db, |sql| skarma_core::store::save_session(sql, id, request))
}
pub fn save_lesson(db: &mut Connection, id: &str, request: SaveLesson) -> Result<Lesson> {
    transaction(db, |sql| skarma_core::store::save_lesson(sql, id, request))
}
pub fn update_follow_up(
    db: &mut Connection,
    table: &str,
    id: &str,
    request: UpdateFollowUp,
) -> Result<FollowUp> {
    transaction(db, |sql| {
        skarma_core::store::update_follow_up(sql, table, id, request)
    })
}
pub fn seed_demo(db: &mut Connection) -> Result<()> {
    transaction(db, skarma_core::store::seed_demo)
}

pub fn bootstrap_admin(
    db: &mut Connection,
    input: skarma_core::auth::NewUser,
) -> Result<skarma_core::auth::User> {
    transaction(db, |sql| skarma_core::auth::bootstrap_admin(sql, input))
}

pub fn recover_admin(db: &mut Connection, username: &str, password: &str) -> Result<()> {
    transaction(db, |sql| {
        skarma_core::auth::recover_admin(sql, username, password)
    })
}

/// Explicit PRD development entry: rejects legacy databases, never called by startup.
pub fn initialize_foundation(db: &Connection) -> Result<()> {
    db.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|_| ApiError::storage())?;
    let tx = rusqlite::Transaction::new_unchecked(db, TransactionBehavior::Immediate)
        .map_err(|_| ApiError::storage())?;
    skarma_core::foundation::initialize(&Sqlite(&tx))?;
    tx.commit().map_err(|_| ApiError::storage())
}
pub fn write_foundation(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    request: skarma_core::foundation::model::Write,
) -> Result<skarma_core::foundation::model::Saved> {
    transaction(db, |sql| {
        skarma_core::foundation::write(sql, actor, request)
    })
}
pub fn foundation_snapshot(
    db: &mut Connection,
) -> Result<skarma_core::foundation::model::Snapshot> {
    transaction(db, skarma_core::foundation::snapshot)
}
pub fn foundation_history(
    db: &mut Connection,
    id: &str,
) -> Result<Vec<skarma_core::foundation::model::Revision>> {
    transaction(db, |sql| skarma_core::foundation::history(sql, id))
}
pub fn foundation_catalog_snapshot(
    db: &mut Connection,
    id: &str,
    kind: skarma_core::foundation::model::CatalogKind,
) -> Result<skarma_core::foundation::model::CatalogSnapshot> {
    transaction(db, |sql| {
        skarma_core::foundation::catalog_snapshot(sql, id, kind)
    })
}

/// Explicitly selected by configuration. Never migrates a legacy domain database.
pub fn initialize_prd_runtime(db: &Connection) -> Result<()> {
    db.execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|_| ApiError::storage())?;
    let tx = rusqlite::Transaction::new_unchecked(db, TransactionBehavior::Immediate)
        .map_err(|_| ApiError::storage())?;
    skarma_core::foundation::initialize_authenticated(&Sqlite(&tx))?;
    tx.commit().map_err(|_| ApiError::storage())
}

pub fn write_foundation_lesson(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    request: skarma_core::foundation::lesson_model::LessonWrite,
) -> Result<skarma_core::foundation::lesson_model::Lesson> {
    transaction(db, |sql| {
        skarma_core::foundation::lessons::write(sql, actor, request)
    })
}
pub fn foundation_lesson_history(
    db: &mut Connection,
    id: &str,
) -> Result<Vec<skarma_core::foundation::lesson_model::LessonRevision>> {
    transaction(db, |sql| skarma_core::foundation::lessons::history(sql, id))
}
pub fn foundation_lesson_withdrawal_capability(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    id: &str,
) -> Result<skarma_core::foundation::lesson_model::WithdrawalCapability> {
    transaction(db, |sql| {
        skarma_core::foundation::lessons::withdrawal_capability(sql, actor, id)
    })
}

pub fn write_foundation_session(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    request: skarma_core::foundation::session_model::Write,
) -> Result<skarma_core::foundation::session_model::Session> {
    transaction(db, |sql| {
        skarma_core::foundation::sessions::write(sql, actor, request)
    })
}
pub fn foundation_coverage_at(
    db: &mut Connection,
    request: skarma_core::foundation::coverage::Request,
    day: &str,
) -> Result<skarma_core::foundation::coverage::Report> {
    transaction(db, |sql| {
        skarma_core::foundation::coverage::report_at(sql, request, day)
    })
}

pub fn foundation_process(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    request: skarma_core::foundation::process_model::Command,
) -> Result<serde_json::Value> {
    transaction(db, |sql| {
        skarma_core::foundation::followups::command(sql, actor, request)
    })
}

/// Offline maintenance/testing entry. Public HTTP always uses authenticated Command.
pub fn data_transfer(
    db: &mut Connection,
    actor: &skarma_core::auth::User,
    request: skarma_core::foundation::transfer::Command,
) -> Result<serde_json::Value> {
    transaction(db, |sql| {
        skarma_core::foundation::transfer::command(sql, actor, request)
    })
}
