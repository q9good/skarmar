pub mod auth;
pub mod config;
pub mod foundation;
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_actor: Option<String>,
        command: Box<Command>,
    },
    Auth(auth::AuthCommand),
    ImportAuthorize,
    ImportPreview(import::ImportPreviewInput),
    ImportCommit(import::ImportCommitInput),
    RuntimeInfo,
    DataTransfer(foundation::transfer::Command),
    FoundationQueued {
        user_id: String,
        write: foundation::lesson_model::QueuedWrite,
    },
    FoundationExportSnapshot,
    FoundationState,
    FoundationProcess(foundation::process_model::Command),
    FoundationSession(foundation::session_model::Write),
    FoundationSessionHistory {
        id: String,
    },
    FoundationCoverage(foundation::coverage::Request),
    FoundationHistory {
        id: String,
    },
    FoundationWrite(foundation::model::Write),
    FoundationLesson(foundation::lesson_model::LessonWrite),
    FoundationLessonHistory {
        id: String,
    },
    FoundationLessonWithdrawalCapability {
        id: String,
    },
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
    let prd = foundation::enabled(db)?;
    if prd
        && !matches!(
            &command,
            Command::RuntimeInfo
                | Command::DataTransfer(_)
                | Command::Authenticated { .. }
                | Command::Auth(_)
                | Command::FoundationQueued { .. }
                | Command::FoundationExportSnapshot
                | Command::FoundationState
                | Command::FoundationHistory { .. }
                | Command::FoundationWrite(_)
                | Command::FoundationLesson(_)
                | Command::FoundationLessonHistory { .. }
                | Command::FoundationLessonWithdrawalCapability { .. }
                | Command::FoundationSession(_)
                | Command::FoundationSessionHistory { .. }
                | Command::FoundationCoverage(_)
                | Command::FoundationProcess(_)
        )
    {
        return Err(store::ApiError::conflict(
            "页面版本与新模型不匹配，请保留填写后刷新页面",
        ));
    }
    if !prd
        && matches!(
            &command,
            Command::FoundationQueued { .. }
                | Command::FoundationExportSnapshot
                | Command::FoundationState
                | Command::FoundationHistory { .. }
                | Command::FoundationWrite(_)
                | Command::FoundationLesson(_)
                | Command::FoundationLessonHistory { .. }
                | Command::FoundationLessonWithdrawalCapability { .. }
                | Command::FoundationSession(_)
                | Command::FoundationSessionHistory { .. }
                | Command::FoundationCoverage(_)
                | Command::FoundationProcess(_)
        )
    {
        return Err(store::ApiError(
            axum::http::StatusCode::NOT_FOUND,
            "尚未启用新模型".into(),
        ));
    }
    Ok(match command {
        Command::Authenticated {
            token,
            command,
            expected_actor,
        } => {
            let actor = auth::session(db, &token)?;
            if expected_actor.as_ref().is_some_and(|id| id != &actor.id) {
                return Err(store::ApiError::conflict(auth::SCOPE_CHANGED));
            }
            if matches!(*command, Command::Auth(_) | Command::Authenticated { .. }) {
                return Err(store::ApiError::invalid("认证请求不能嵌套"));
            }
            if prd
                && !matches!(
                    &*command,
                    Command::RuntimeInfo
                        | Command::DataTransfer(_)
                        | Command::FoundationQueued { .. }
                        | Command::FoundationExportSnapshot
                        | Command::FoundationState
                        | Command::FoundationHistory { .. }
                        | Command::FoundationWrite(_)
                        | Command::FoundationLesson(_)
                        | Command::FoundationLessonHistory { .. }
                        | Command::FoundationLessonWithdrawalCapability { .. }
                        | Command::FoundationSession(_)
                        | Command::FoundationSessionHistory { .. }
                        | Command::FoundationCoverage(_)
                        | Command::FoundationProcess(_)
                )
            {
                return Err(store::ApiError::conflict(
                    "页面版本与新模型不匹配，请保留填写后刷新页面",
                ));
            }
            if !prd
                && matches!(
                    &*command,
                    Command::FoundationQueued { .. }
                        | Command::FoundationExportSnapshot
                        | Command::FoundationState
                        | Command::FoundationHistory { .. }
                        | Command::FoundationWrite(_)
                        | Command::FoundationLesson(_)
                        | Command::FoundationLessonHistory { .. }
                        | Command::FoundationLessonWithdrawalCapability { .. }
                        | Command::FoundationSession(_)
                        | Command::FoundationSessionHistory { .. }
                        | Command::FoundationCoverage(_)
                        | Command::FoundationProcess(_)
                )
            {
                return Err(store::ApiError(
                    axum::http::StatusCode::NOT_FOUND,
                    "尚未启用新模型".into(),
                ));
            }
            match *command {
                Command::DataTransfer(request) => {
                    foundation::transfer::command(db, &actor, request)?
                }
                Command::FoundationQueued { user_id, write } => {
                    if actor.id != user_id {
                        return Err(store::ApiError::conflict(
                            "账号已变更，原账号的待上传记录已暂停",
                        ));
                    }
                    match write {
                        foundation::lesson_model::QueuedWrite::Catalog(request) => {
                            if !matches!(request.mutation, foundation::model::Mutation::Catalog(_))
                                || request.expected_version.is_some()
                            {
                                return Err(store::ApiError::invalid(
                                    "上传队列仅支持新增课程或老师",
                                ));
                            }
                            serde_json::to_value(foundation::write(db, &actor, request)?)?
                        }
                        foundation::lesson_model::QueuedWrite::Lesson(request) => {
                            if !matches!(
                                request.action,
                                foundation::lesson_model::LessonAction::Observation(_)
                            ) || request.expected_version.is_some()
                            {
                                return Err(store::ApiError::invalid("上传队列仅支持新增基础课记"));
                            }
                            serde_json::to_value(foundation::lessons::write(db, &actor, request)?)?
                        }
                    }
                }
                Command::FoundationProcess(request) => {
                    foundation::followups::command(db, &actor, request)?
                }
                Command::FoundationExportSnapshot => {
                    serde_json::to_value(foundation::export::snapshot(db)?)?
                }
                Command::FoundationState => serde_json::to_value(foundation::snapshot(db)?)?,
                Command::FoundationHistory { id } => {
                    serde_json::to_value(foundation::history(db, &id)?)?
                }
                Command::FoundationLesson(request) => {
                    serde_json::to_value(foundation::lessons::write(db, &actor, request)?)?
                }
                Command::FoundationLessonHistory { id } => {
                    serde_json::to_value(foundation::lessons::history(db, &id)?)?
                }
                Command::FoundationLessonWithdrawalCapability { id } => serde_json::to_value(
                    foundation::lessons::withdrawal_capability(db, &actor, &id)?,
                )?,
                Command::FoundationSession(request) => {
                    serde_json::to_value(foundation::sessions::write(db, &actor, request)?)?
                }
                Command::FoundationSessionHistory { id } => {
                    serde_json::to_value(foundation::sessions::history(db, &id)?)?
                }
                Command::FoundationCoverage(request) => {
                    serde_json::to_value(foundation::coverage::report(db, request)?)?
                }
                Command::FoundationWrite(request) => {
                    serde_json::to_value(foundation::write(db, &actor, request)?)?
                }
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
        Command::DataTransfer(_) => return Err(auth::unauthorized()),
        Command::RuntimeInfo => {
            serde_json::json!({"model":if prd {"prd_v1"} else {"legacy"},"contract":if prd {"skarmar.prd.v1"} else {"legacy"}})
        }
        Command::FoundationQueued { .. }
        | Command::FoundationExportSnapshot
        | Command::FoundationState
        | Command::FoundationHistory { .. }
        | Command::FoundationWrite(_)
        | Command::FoundationLesson(_)
        | Command::FoundationLessonHistory { .. }
        | Command::FoundationLessonWithdrawalCapability { .. }
        | Command::FoundationSession(_)
        | Command::FoundationSessionHistory { .. }
        | Command::FoundationCoverage(_)
        | Command::FoundationProcess(_) => return Err(auth::unauthorized()),
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
