//! Synthetic classroom/reference, priority and due-schedule contracts only.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::auth::{AccountRole, User};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn fixture() -> (Connection, User, String, String, String) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let u = User {
        id: id(),
        username: "synthetic".into(),
        display_name: "合成家人".into(),
        role: AccountRole::Admin,
        active: true,
    };
    let g = put(
        &mut db,
        &u,
        "goal",
        json!({"title":"合成阶段目标","level":"short"}),
    );
    let c = put(
        &mut db,
        &u,
        "catalog",
        json!({"kind":"course","name":"合成课程","active":true}),
    );
    let t = put(
        &mut db,
        &u,
        "catalog",
        json!({"kind":"teacher","name":"合成老师","active":true}),
    );
    (db, u, g, c, t)
}
fn put(db: &mut Connection, u: &User, kind: &str, value: Value) -> String {
    let i = id();
    store::write_foundation(db,u,serde_json::from_value(json!({"id":i,"operation_id":id(),"expected_version":null,"change_note":"","mutation":{"kind":kind,"value":value}})).unwrap()).unwrap();
    i
}
fn observation(c: &str, t: &str, notes: &str) -> Value {
    json!({"business_date":"2020-01-01","course_id":c,"teacher_id":t,"notes":notes})
}
fn lesson(
    db: &mut Connection,
    u: &User,
    l: &str,
    version: Option<u64>,
    kind: &str,
    value: Value,
) -> store::Result<Value> {
    store::write_foundation_lesson(db,u,serde_json::from_value(json!({"id":l,"operation_id":id(),"expected_version":version,"action":{"kind":kind,"value":value}})).unwrap()).map(|v|serde_json::to_value(v).unwrap())
}
fn process(db: &mut Connection, u: &User, kind: &str, value: Value) -> store::Result<Value> {
    store::foundation_process(
        db,
        u,
        serde_json::from_value(json!({"kind":kind,"value":value})).unwrap(),
    )
}
#[test]
fn observation_supplements_have_no_revision_feed_but_intermediate_references_stay_immutable() {
    let (mut db, u, g, c, t) = fixture();
    let l = id();
    lesson(
        &mut db,
        &u,
        &l,
        None,
        "observation",
        observation(&c, &t, "合成最初观察"),
    )
    .unwrap();
    lesson(
        &mut db,
        &u,
        &l,
        Some(1),
        "observation",
        observation(&c, &t, "合成整理所据中间观察"),
    )
    .unwrap();
    lesson(
        &mut db,
        &u,
        &l,
        Some(2),
        "observation",
        observation(&c, &t, "合成后来观察"),
    )
    .unwrap();
    assert_eq!(
        store::foundation_lesson_history(&mut db, &l).unwrap().len(),
        1
    );
    let sid = id();
    let saved=store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":{"business_date":"2020-01-01","time_slot":"上午","lesson_sources":[{"id":l,"version":2}],"summary":{"targets":[{"goal_id":g,"performed":true,"observation":"合成家庭表现"}],"confirmation":"none"}}})).unwrap()).unwrap();
    assert_eq!(
        saved.lesson_sources[0].observation.notes,
        "合成整理所据中间观察"
    );
    let kid = id();
    let organization = json!({"seen_observation_version":2,"targets":[{"goal_id":g,"note":"合成明确课上涉及"}],"new_knowledge":[{"id":kid,"kind":"difficulty","content":"合成困难","goal_ids":[g]}]});
    lesson(&mut db, &u, &l, Some(1), "organization", organization).unwrap();
    lesson(
        &mut db,
        &u,
        &l,
        Some(3),
        "observation",
        observation(&c, &t, "合成最终观察"),
    )
    .unwrap();
    let state = store::foundation_snapshot(&mut db).unwrap();
    let k = state.knowledge.iter().find(|k| k.input.id == kid).unwrap();
    assert_eq!(
        k.source_observation.as_ref().unwrap().notes,
        "合成整理所据中间观察"
    );
    assert_eq!(
        state
            .sessions
            .iter()
            .find(|s| s.id == sid)
            .unwrap()
            .lesson_sources[0]
            .observation
            .notes,
        "合成整理所据中间观察"
    );
    assert_eq!(state.lessons[0].observation.notes, "合成最终观察");
    assert_eq!(
        store::foundation_lesson_history(&mut db, &l)
            .unwrap()
            .iter()
            .filter(|r| r.event == "observation")
            .count(),
        1
    );
    assert!(
        lesson(
            &mut db,
            &u,
            &l,
            Some(2),
            "observation",
            observation(&c, &t, "合成冲突修改")
        )
        .is_err()
    );
    assert!(
        db.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none()
    );
}
#[test]
fn observation_updates_automatically_request_family_review_and_reopen_processed_work() {
    let (mut db, u, _g, c, t) = fixture();
    let l = id();
    let blank = lesson(
        &mut db,
        &u,
        &l,
        None,
        "observation",
        observation(&c, &t, ""),
    )
    .unwrap();
    assert_eq!(blank["request_generation"], 0);
    let meaningful = lesson(
        &mut db,
        &u,
        &l,
        Some(1),
        "observation",
        observation(&c, &t, "合成补充观察"),
    )
    .unwrap();
    assert_eq!(meaningful["request_generation"], 1);
    let board = process(&mut db, &u, "read", json!({"view":"board"})).unwrap();
    assert_eq!(board["inbox"]["requests"].as_array().unwrap().len(), 1);
    lesson(
        &mut db,
        &u,
        &l,
        Some(1),
        "organization",
        json!({"seen_observation_version":2,"processing":"complete","request_generation":1}),
    )
    .unwrap();
    assert!(
        process(&mut db, &u, "read", json!({"view":"board"})).unwrap()["inbox"]["requests"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let unchanged = lesson(
        &mut db,
        &u,
        &l,
        Some(2),
        "observation",
        observation(&c, &t, "合成补充观察"),
    )
    .unwrap();
    assert_eq!(unchanged["request_generation"], 1);
    assert!(
        process(&mut db, &u, "read", json!({"view":"board"})).unwrap()["inbox"]["requests"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let changed = lesson(
        &mut db,
        &u,
        &l,
        Some(3),
        "observation",
        observation(&c, &t, "合成新建议"),
    )
    .unwrap();
    assert_eq!(changed["request_generation"], 2);
    let board = process(&mut db, &u, "read", json!({"view":"board"})).unwrap();
    assert_eq!(board["inbox"]["requests"][0]["id"], l);
    assert_eq!(board["inbox"]["requests"][0]["claimed_by"], Value::Null);
    let stale = lesson(
        &mut db,
        &u,
        &l,
        Some(2),
        "organization",
        json!({"seen_observation_version":3,"processing":"complete","request_generation":1}),
    )
    .unwrap_err();
    assert_eq!(stale.0, axum::http::StatusCode::CONFLICT);
    assert_eq!(
        process(&mut db, &u, "read", json!({"view":"board"})).unwrap()["inbox"]["requests"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        lesson(
            &mut db,
            &u,
            &l,
            Some(2),
            "organization",
            json!({"seen_observation_version":3,"processing":"complete","request_generation":2})
        )
        .is_err()
    );
    lesson(
        &mut db,
        &u,
        &l,
        Some(2),
        "organization",
        json!({"seen_observation_version":4,"processing":"complete","request_generation":2}),
    )
    .unwrap();
    assert!(
        process(&mut db, &u, "read", json!({"view":"board"})).unwrap()["inbox"]["requests"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn priorities_round_trip_and_older_client_edits_preserve_new_method_priority() {
    let (mut db, u, g, _c, _t) = fixture();
    let mid = id();
    let value = json!({"title":"合成方法","steps":"合成明确步骤","goal_ids":[g]});
    let command = |operation: String, expected: Option<u64>, input: Value| {
        serde_json::from_value(json!({"id":mid,"operation_id":operation,"expected_version":expected,"method_create_mode":if expected.is_none(){json!("full")}else{Value::Null},"change_note":"","mutation":{"kind":"method","value":input}})).unwrap()
    };
    store::write_foundation(&mut db, &u, command(id(), None, value.clone())).unwrap();
    let mut high = value.clone();
    high["priority"] = json!(2);
    store::write_foundation(&mut db, &u, command(id(), Some(1), high)).unwrap();
    let mut old_client = value;
    old_client["title"] = json!("合成只改名称");
    let result = store::write_foundation(&mut db, &u, command(id(), Some(2), old_client)).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["value"]["input"]["priority"],
        2
    );
    let bad = json!({"title":"合成","steps":"合成步骤","goal_ids":[g],"priority":3});
    assert!(store::write_foundation(&mut db, &u, command(id(), Some(3), bad)).is_err());
    let goal=store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":1,"change_note":"","mutation":{"kind":"goal","value":{"title":"合成阶段目标","level":"short","long_priority":2}}})).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(goal).unwrap()["value"]["input"]["long_priority"],
        2
    );
}
#[test]
fn due_schedule_is_returned_on_its_day_without_writing_or_resetting_the_object() {
    let (mut db, u, g, _c, _t) = fixture();
    let today = skarma_core::foundation::sessions::today();
    db.execute("UPDATE prd_schedules SET data=json_set(data,'$.due_date',?2) WHERE object_kind='goal' AND object_id=?1",[&g,&today]).unwrap();
    let before = store::foundation_snapshot(&mut db).unwrap();
    let board = process(&mut db, &u, "read", json!({"view":"board"})).unwrap();
    let s = board["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["object_id"] == g)
        .unwrap();
    assert_eq!(s["due_date"], today);
    assert_eq!(s["reminders"], true);
    assert_eq!(s["eligible"], true);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(store::foundation_snapshot(&mut db).unwrap()).unwrap()
    );
}
