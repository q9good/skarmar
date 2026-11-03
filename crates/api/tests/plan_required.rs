//! Synthetic-only regression: required plan text does not rewrite existing data.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, NewUser},
    foundation::session_model::{Session, Write},
};
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn fixture() -> (Connection, skarma_core::auth::User) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    let actor = store::bootstrap_admin(
        &mut db,
        NewUser {
            username: "synthetic-owner".into(),
            display_name: "合成家长".into(),
            password: "synthetic-only".into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    (db, actor)
}
fn plan(text: &str) -> Write {
    serde_json::from_value(json!({"id":id(),"operation_id":id(),"expected_version":null,"action":"save_draft","input":{"business_date":"2020-01-01","time_slot":"下午","plan":text},"change_note":""})).unwrap()
}
#[test]
fn a_shared_plan_requires_activity_text_and_rejection_has_no_partial_write() {
    let (mut db, actor) = fixture();
    for text in ["", " \t\n", "\u{3000}"] {
        let error = store::write_foundation_session(&mut db, &actor, plan(text)).unwrap_err();
        assert_eq!(error.0, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert!(error.1.contains("准备做什么"));
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_sessions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_operations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let request = plan("一起玩轮流滚球，等待孩子主动求助");
    let saved = store::write_foundation_session(&mut db, &actor, request.clone()).unwrap();
    assert_eq!(saved.input.plan, "一起玩轮流滚球，等待孩子主动求助");
    assert!(
        saved.input.targets.is_empty(),
        "only the activity was made required"
    );
    assert_eq!(
        store::write_foundation_session(&mut db, &actor, request)
            .unwrap()
            .version,
        1
    );
}
#[test]
fn old_blank_plan_receipts_history_and_summary_drafts_are_not_retroactively_rejected() {
    let (mut db, actor) = fixture();
    let mut original = plan("旧版本合成安排");
    let mut saved = store::write_foundation_session(&mut db, &actor, original.clone()).unwrap();
    // Construct the pre-change stored request/receipt without using a production migration.
    original.input.as_mut().unwrap().plan.clear();
    saved.input.plan.clear();
    let data = serde_json::to_string(&saved).unwrap();
    db.execute(
        "UPDATE prd_sessions SET data=?1 WHERE id=?2",
        [&data, &saved.id],
    )
    .unwrap();
    db.execute(
        "UPDATE prd_operations SET request=?1,response=?2 WHERE id=?3",
        [
            format!("session:{}", serde_json::to_string(&original).unwrap()),
            data.clone(),
            original.operation_id.clone(),
        ],
    )
    .unwrap();
    let revision: String = db
        .query_row(
            "SELECT data FROM prd_session_revisions WHERE session_id=?1",
            [&saved.id],
            |r| r.get(0),
        )
        .unwrap();
    let mut revision: Value = serde_json::from_str(&revision).unwrap();
    revision["session"]["input"]["plan"] = json!("");
    db.execute(
        "UPDATE prd_session_revisions SET data=?1 WHERE session_id=?2",
        [revision.to_string(), saved.id.clone()],
    )
    .unwrap();
    let replay = store::write_foundation_session(&mut db, &actor, original).unwrap();
    assert_eq!(replay.input.plan, "");
    assert_eq!(replay.version, 1);
    let snapshot = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(snapshot.sessions[0].input.plan, "");
    let input = json!({"business_date":"2020-01-01","time_slot":"下午","plan":"","summary":{"targets":[],"confirmation":"unconfirmed","methods":[]}});
    let request:Write=serde_json::from_value(json!({"id":saved.id,"operation_id":id(),"expected_version":1,"action":"save_draft","input":input,"change_note":""})).unwrap();
    let draft: Session = store::write_foundation_session(&mut db, &actor, request).unwrap();
    assert_eq!(draft.status, "summary_draft");
    assert_eq!(draft.input.plan, "");
    let request:Write=serde_json::from_value(json!({"id":draft.id,"operation_id":id(),"expected_version":2,"action":"cancel","input":null,"change_note":"取消合成草稿"})).unwrap();
    assert_eq!(
        store::write_foundation_session(&mut db, &actor, request)
            .unwrap()
            .status,
        "cancelled"
    );
}
