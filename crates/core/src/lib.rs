pub mod config;
pub mod model;
pub mod sql;
pub mod store;

use model::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sql::Sql;
use store::Result;

/// Internal command transport, shared by both runtimes. Not a public HTTP endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "command", content = "input", rename_all = "snake_case")]
pub enum Command {
    State,
    GoalHistory {
        id: String,
    },
    CreateGoal(NewGoal),
    UpdateGoal {
        id: String,
        request: UpdateGoal,
    },
    SaveSession {
        id: String,
        request: SaveSession,
    },
    UpdateFollowUp {
        table: String,
        id: String,
        request: UpdateFollowUp,
    },
}

/// The adapter must enclose this entire call in one database transaction.
/// Returning Err must roll back all writes, including goal revisions and operation IDs.
pub fn execute(db: &dyn Sql, command: Command) -> Result<Value> {
    Ok(match command {
        Command::State => serde_json::to_value(store::state(db)?)?,
        Command::GoalHistory { id } => serde_json::to_value(store::goal_history(db, &id)?)?,
        Command::CreateGoal(input) => serde_json::to_value(store::create_goal(db, input)?)?,
        Command::UpdateGoal { id, request } => {
            serde_json::to_value(store::update_goal(db, &id, request)?)?
        }
        Command::SaveSession { id, request } => {
            serde_json::to_value(store::save_session(db, &id, request)?)?
        }
        Command::UpdateFollowUp { table, id, request } => {
            serde_json::to_value(store::update_follow_up(db, &table, &id, request)?)?
        }
    })
}
