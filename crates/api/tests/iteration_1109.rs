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
fn numeric_policy_is_shared_versioned_and_replayed_and_old_edit_keeps_score() {
    let (mut db, u, g, _, _) = fixture();
    let policy =
        json!({"version":0,"minimum":0,"maximum":200,"method_per_goal":3,"method_bonus_cap":30});
    let command = json!({"operation_id":id(),"expected_version":0,"value":policy});
    let first = process(&mut db, &u, "priority_settings", command.clone()).unwrap();
    assert_eq!(first["version"], 1);
    assert_eq!(
        process(&mut db, &u, "priority_settings", command.clone()).unwrap(),
        first
    );
    assert!(
        process(
            &mut db,
            &u,
            "priority_settings",
            json!({"operation_id":id(),"expected_version":0,"value":policy})
        )
        .is_err()
    );
    let current = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(current.priority_policy.maximum, 200);
    assert!(current.created_at.contains_key(&g));
    let saved=store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":1,"change_note":"","mutation":{"kind":"goal","value":{"title":"合成150分","level":"short","priority_score":150,"priority_until":"2099-01-01"}}})).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(saved).unwrap()["value"]["input"]["priority_score"],
        150
    );
    let saved=store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":2,"change_note":"旧客户端更正文案","mutation":{"kind":"goal","value":{"title":"合成旧端改名","level":"short"}}})).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(saved).unwrap()["value"]["input"]["priority_score"],
        150
    );
    assert!(store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":3,"change_note":"","mutation":{"kind":"goal","value":{"title":"合成越界","level":"short","priority_score":201}}})).unwrap()).is_err());
}
#[test]
fn observation_items_keep_ids_replay_projection_legacy_edits_and_sources() {
    let (mut db, u, _, c, t) = fixture();
    let l = id();
    let a = id();
    let b = id();
    let op = id();
    let input = json!({"business_date":"2020-01-01","course_id":c,"teacher_id":t,"items":[{"id":a,"kind":"teacher_notes","content":"合成第一件事\n仍然是同一条"},{"id":b,"kind":"teacher_notes","content":"合成第二件事"}]});
    let req = json!({"id":l,"operation_id":op,"expected_version":null,"action":{"kind":"observation","value":input}});
    let first =
        store::write_foundation_lesson(&mut db, &u, serde_json::from_value(req.clone()).unwrap())
            .unwrap();
    assert_eq!(first.observation.items.as_ref().unwrap().len(), 2);
    assert_eq!(
        serde_json::to_value(
            store::write_foundation_lesson(&mut db, &u, serde_json::from_value(req).unwrap())
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    let mut old = serde_json::to_value(&first.observation).unwrap();
    old.as_object_mut().unwrap().remove("items");
    old["interest_notes"] = json!("合成旧端新增兴趣");
    let second = lesson(&mut db, &u, &l, Some(1), "observation", old).unwrap();
    assert_eq!(second["observation"]["items"][0]["id"], a);
    assert_eq!(second["observation"]["items"][1]["id"], b);
    assert_eq!(second["observation"]["items"].as_array().unwrap().len(), 3);
    let mut full_old=second["observation"].clone();
    full_old["helpful_notes"]=json!("合成旧端保留items但编辑扁平字段");
    let third=lesson(&mut db,&u,&l,Some(2),"observation",full_old).unwrap();
    assert_eq!(third["observation"]["items"][0]["id"],a);
    assert_eq!(third["observation"]["helpful_notes"],"合成旧端保留items但编辑扁平字段");
    let before = store::foundation_snapshot(&mut db).unwrap();
    assert!(lesson(&mut db,&u,&l,Some(3),"observation",json!({"business_date":"2020-01-01","course_id":c,"teacher_id":t,"items":[{"id":a,"kind":"teacher_notes","content":"一"},{"id":a,"kind":"teacher_notes","content":"二"}]})).is_err());
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(store::foundation_snapshot(&mut db).unwrap()).unwrap()
    );
}
#[test]
fn closing_review_keeps_lesson_and_new_material_reopens_without_deleting_data() {
    let (mut db, u, g, c, t) = fixture();
    let l = id();
    let first = lesson(
        &mut db,
        &u,
        &l,
        None,
        "observation",
        observation(&c, &t, "合成重复材料"),
    )
    .unwrap();
    let closed=lesson(&mut db,&u,&l,Some(1),"organization",json!({"seen_observation_version":1,"request_generation":first["request_generation"],"processing":"complete","outcome":"关闭：重复材料","targets":[{"goal_id":g}]})).unwrap();
    assert_eq!(closed["organization"]["state"], "done");
    assert_eq!(closed["status"], "recorded");
    let next = lesson(
        &mut db,
        &u,
        &l,
        Some(1),
        "observation",
        observation(&c, &t, "合成新的观察"),
    )
    .unwrap();
    assert!(
        next["request_generation"].as_u64().unwrap()
            > closed["request_generation"].as_u64().unwrap()
    );
    assert_eq!(
        next["organization"]["targets"],
        closed["organization"]["targets"]
    );
    assert!(lesson(&mut db,&u,&l,Some(2),"organization",json!({"seen_observation_version":1,"request_generation":next["request_generation"],"processing":"complete"})).is_err());
}
