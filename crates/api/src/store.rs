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
