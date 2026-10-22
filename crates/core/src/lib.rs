pub mod auth;
pub mod config;
pub mod import;
pub mod lessons;
pub mod model;
pub mod mutations;
pub mod sql;
pub mod store;

use model::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sql::Sql;
use store::Result;

/// Internal command transport, shared by both runtimes. Not a public HTTP endpoint.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "command", content = "input", rename_all = "snake_case")]
pub enum Command {
    Authenticated {
        token: String,
        command: Box<Command>,
    },
    Auth(auth::AuthCommand),
    ImportAuthorize,
    ImportPreview(import::ImportPreviewInput),
    ImportCommit(import::ImportCommitInput),
    State,
    ExportSnapshot,
    GoalHistory {
        id: String,
    },
    SessionHistory {
        id: String,
    },
    DeleteRecord {
        kind: String,
        id: String,
        request: DeleteRecord,
    },
    CreateGoal(NewGoal),
    CreatePathway(NewPathway),
    UpdatePathway {
        id: String,
        request: UpdatePathway,
    },
    CreateExperience(NewExperience),
    UpdateGoal {
        id: String,
        request: UpdateGoal,
    },
    SaveSession {
        id: String,
        request: SaveSession,
    },
    SaveLesson {
        id: String,
        request: SaveLesson,
    },
    AddLessonOption(NewLessonOption),
    UpdateLessonOption(UpdateLessonOption),
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
        Command::Authenticated { token, command } => {
            let actor = auth::session(db, &token)?;
            if matches!(*command, Command::Auth(_) | Command::Authenticated { .. }) {
                return Err(store::ApiError::invalid("认证请求不能嵌套"));
            }
            match *command {
                Command::DeleteRecord { kind, id, request } => {
                    if actor.role != auth::AccountRole::Admin {
                        return Err(store::ApiError(
                            axum::http::StatusCode::FORBIDDEN,
                            "只有管理员可以删除记录".into(),
                        ));
                    }
                    mutations::delete_record(db, &kind, &id, request, &actor.id)?;
                    serde_json::json!({"ok": true})
                }
                Command::ImportAuthorize => {
                    import::ensure_admin(&actor)?;
                    serde_json::json!({"ok": true})
                }
                Command::ImportPreview(input) => {
                    serde_json::to_value(import::preview(db, &actor, input)?)?
                }
                Command::ImportCommit(input) => {
                    serde_json::to_value(import::commit(db, &actor, input)?)?
                }
                command => execute(db, command)?,
            }
        }
        Command::Auth(command) => auth::execute(db, command)?,
        Command::ImportAuthorize | Command::ImportPreview(_) | Command::ImportCommit(_) => {
            return Err(auth::unauthorized());
        }
        Command::DeleteRecord { .. } => return Err(auth::unauthorized()),
        Command::State => serde_json::to_value(store::state(db)?)?,
        Command::ExportSnapshot => serde_json::to_value(store::export_snapshot(db)?)?,
        Command::GoalHistory { id } => serde_json::to_value(store::goal_history(db, &id)?)?,
        Command::SessionHistory { id } => {
            serde_json::to_value(mutations::session_history(db, &id)?)?
        }
        Command::CreateGoal(input) => serde_json::to_value(store::create_goal(db, input)?)?,
        Command::CreatePathway(input) => serde_json::to_value(store::create_pathway(db, input)?)?,
        Command::UpdatePathway { id, request } => {
            serde_json::to_value(store::update_pathway(db, &id, request)?)?
        }
        Command::CreateExperience(input) => {
            serde_json::to_value(store::create_experience(db, input)?)?
        }
        Command::UpdateGoal { id, request } => {
            serde_json::to_value(store::update_goal(db, &id, request)?)?
        }
        Command::SaveSession { id, request } => {
            serde_json::to_value(store::save_session(db, &id, request)?)?
        }
        Command::SaveLesson { id, request } => {
            serde_json::to_value(store::save_lesson(db, &id, request)?)?
        }
        Command::AddLessonOption(input) => {
            serde_json::to_value(store::add_lesson_option(db, input)?)?
        }
        Command::UpdateLessonOption(input) => {
            serde_json::to_value(store::update_lesson_option(db, input)?)?
        }
        Command::UpdateFollowUp { table, id, request } => {
            serde_json::to_value(store::update_follow_up(db, &table, &id, request)?)?
        }
    })
}
