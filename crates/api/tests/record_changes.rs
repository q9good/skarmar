use rusqlite::Connection;
use skarma_api::{model::*, store};
use skarma_core::Command;
use uuid::Uuid;

#[test]
fn imported_completed_record_can_be_revised_without_inventing_activities_or_goals() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let id = Uuid::new_v4().to_string();
    let original: Session = serde_json::from_value(serde_json::json!({
        "id": id, "version": 1, "status": "completed", "business_date": "2026-10-19", "time_slot": "",
        "activities": [], "organization": "", "plan": "导入原文", "observation": "历史观察", "targets": [], "target_snapshots": [],
        "has_difficulty": false, "difficulty": "", "has_experience": false, "experience": "", "experience_kind": ""
    })).unwrap();
    db.execute(
        "INSERT INTO sessions(id,data) VALUES (?1,?2)",
        [&id, &serde_json::to_string(&original).unwrap()],
    )
    .unwrap();
    let mut input = original.input.clone();
    input.observation = "修正历史观察".into();
    let request = SaveSession {
        operation_id: Uuid::new_v4().to_string(),
        expected_version: Some(1),
        action: "revise".into(),
        session: input,
        revision: Some(SessionRevisionInput {
            note: "更正导入笔误".into(),
            difficulty_version: None,
            experience_version: None,
        }),
    };
    let result = store::save_session(&mut db, &id, request).unwrap();
    assert_eq!(result.session.version, 2);
    assert!(result.session.input.targets.is_empty());
    assert!(result.session.input.activities.is_empty());
    assert!(result.session.input.organization.is_empty());
    assert_eq!(result.session.input.observation, "修正历史观察");
    let history = store::execute(&mut db, Command::SessionHistory { id }).unwrap();
    assert_eq!(history.as_array().unwrap().len(), 2);
    assert_eq!(history[1]["session"]["observation"], "历史观察");
}
