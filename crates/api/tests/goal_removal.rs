//! All records are synthetic. No real account, names, IDs or databases.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::{goal_removal as removal, model::*},
    model::Level,
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn setup() -> (Connection, User) {
    let db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    db.execute_batch("CREATE TABLE auth_users(id TEXT PRIMARY KEY, active INTEGER,role TEXT); INSERT INTO auth_users VALUES('synthetic-admin',1,'admin')").unwrap();
    let u = User {
        id: id(),
        username: "synthetic".into(),
        display_name: "合成管理员".into(),
        role: AccountRole::Admin,
        active: true,
    };
    (db, u)
}
fn put(db: &mut Connection, u: &User, title: &str, level: Level, parent: Option<&str>) -> Saved {
    store::write_foundation(
        db,
        u,
        Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: id(),
            operation_id: id(),
            expected_version: None,
            change_note: "合成创建".into(),
            mutation: Mutation::Goal(
                serde_json::from_value(json!({"title":title,"level":level,"parent_id":parent}))
                    .unwrap(),
            ),
        },
    )
    .unwrap()
}
fn session_history(db: &Connection, id: &str) -> rusqlite::Result<Vec<String>> {
    db.prepare("SELECT data FROM prd_session_revisions WHERE session_id=?1 ORDER BY version")?
        .query_map([id], |r| r.get(0))?
        .collect()
}
fn state(db: &mut Connection) -> Value {
    serde_json::to_value(store::foundation_snapshot(db).unwrap()).unwrap()
}
#[test]
fn removes_only_reviewed_catalogue_members_keeps_entities_history_accounts_and_replays() {
    let (mut db, u) = setup();
    let a = put(&mut db, &u, "合成待移除甲", Level::Short, None);
    let b = put(&mut db, &u, "合成待移除乙", Level::Medium, None);
    let c = put(&mut db, &u, "合成保留长期", Level::Long, None);
    let before = state(&mut db);
    let entities: Vec<String> = db
        .prepare("SELECT data FROM prd_entities ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(store::remove_selected_goals(&mut db, "wrong").is_err());
    assert_eq!(state(&mut db), before);
    assert_eq!(
        store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap(),
        json!({"batch":"synthetic-goals-20261105","completed":true})
    );
    let after = state(&mut db);
    assert_eq!(after["goals"].as_array().unwrap().len(), 1);
    assert_eq!(after["goals"][0]["id"], c.id());
    assert_eq!(after["removed_goal_ids"].as_array().unwrap().len(), 2);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM auth_users", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.prepare("SELECT data FROM prd_entities ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        entities
    );
    assert_eq!(store::foundation_history(&mut db, a.id()).unwrap().len(), 1);
    assert_eq!(store::foundation_history(&mut db, b.id()).unwrap().len(), 1);
    store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap();
    assert_eq!(state(&mut db), after);
    let Saved::Goal(g) = a else { panic!() };
    assert!(
        store::write_foundation(
            &mut db,
            &u,
            Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id: g.id,
                operation_id: id(),
                expected_version: Some(1),
                change_note: "禁止恢复".into(),
                mutation: Mutation::Goal(g.input)
            }
        )
        .is_err()
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_schedules WHERE object_kind='goal'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
#[test]
fn missing_ambiguous_referenced_or_wrong_level_candidates_make_no_changes() {
    for case in ["missing", "duplicate", "child", "long", "method"] {
        let (mut db, u) = setup();
        let a = put(
            &mut db,
            &u,
            "合成待移除甲",
            if case == "long" {
                Level::Long
            } else {
                Level::Medium
            },
            None,
        );
        if case != "missing" {
            put(&mut db, &u, "合成待移除乙", Level::Short, None);
        }
        if case == "duplicate" {
            put(&mut db, &u, "合成待移除甲", Level::Short, None);
        }
        if case == "child" {
            put(&mut db, &u, "合成保留子目标", Level::Short, Some(a.id()));
        }
        if case == "method" {
            store::write_foundation(
                &mut db,
                &u,
                Write {
                    provided_fields: None,
                    method_change: None,
                    method_create_mode: None,
                    id: id(),
                    operation_id: id(),
                    expected_version: None,
                    change_note: "".into(),
                    mutation: Mutation::Method(
                        serde_json::from_value(
                            json!({"title":"合成仍关联的方法","goal_ids":[a.id()]}),
                        )
                        .unwrap(),
                    ),
                },
            )
            .unwrap();
        }
        let before = state(&mut db);
        assert_eq!(
            store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap()["completed"],
            false,
            "{case}"
        );
        assert_eq!(state(&mut db), before, "{case}");
        let result = store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap();
        let reason = match case {
            "missing" | "long" => "candidate_missing",
            "duplicate" => "candidate_ambiguous",
            _ => "linked_entities",
        };
        assert_eq!(
            result,
            json!({"batch":"synthetic-goals-20261105","completed":false,"blocker":reason})
        );
    }
}

#[test]
fn late_marker_failure_rolls_back_catalogue_and_schedule_changes() {
    let (mut db, u) = setup();
    put(&mut db, &u, "合成待移除甲", Level::Short, None);
    put(&mut db, &u, "合成待移除乙", Level::Medium, None);
    let before = state(&mut db);
    db.execute_batch("CREATE TRIGGER removal_failure BEFORE INSERT ON prd_upgrade WHEN new.id='goal-removal-20261105' BEGIN SELECT RAISE(ABORT,'synthetic marker failure'); END").unwrap();
    assert!(store::remove_selected_goals(&mut db, removal::TEST_ACK).is_err());
    assert_eq!(state(&mut db), before);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_schedules WHERE object_kind='goal'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
}

#[test]
fn referenced_session_snapshots_survive_removal_and_can_be_revised_but_not_reused() {
    use skarma_core::foundation::session_model as session;
    let (mut db, u) = setup();
    let parent = put(&mut db, &u, "合成保留上级", Level::Long, None);
    let a = put(
        &mut db,
        &u,
        "合成待移除甲",
        Level::Medium,
        Some(parent.id()),
    );
    put(&mut db, &u, "合成待移除乙", Level::Short, None);
    let sid = id();
    let input:session::Input=serde_json::from_value(json!({"business_date":"2020-01-01","time_slot":"晚上","summary":{"targets":[{"goal_id":a.id(),"performed":true,"observation":"合成原观察"}],"confirmation":"none"}})).unwrap();
    let mut input = input;
    let aid = id();
    let kid = id();
    input.summary.as_mut().unwrap().new_actions = vec![
        serde_json::from_value(json!({"id":aid,"content":"合成保留下一步","goal_ids":[a.id()]}))
            .unwrap(),
    ];
    input.summary.as_mut().unwrap().new_knowledge=vec![serde_json::from_value(json!({"id":kid,"kind":"experience","content":"合成保留经验","stage":"try","goal_ids":[a.id()]})).unwrap()];
    let first = store::write_foundation_session(
        &mut db,
        &u,
        session::Write {
            id: sid.clone(),
            operation_id: id(),
            expected_version: None,
            action: session::Action::Complete,
            input: Some(input.clone()),
            change_note: "".into(),
        },
    )
    .unwrap();
    let before = state(&mut db);
    let history_before = serde_json::to_value(session_history(&db, &sid).unwrap()).unwrap();
    assert_eq!(
        store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap()["completed"],
        true
    );
    let after = state(&mut db);
    assert_eq!(after["sessions"], before["sessions"]);
    assert_eq!(after["knowledge"], before["knowledge"]);
    assert_eq!(after["actions"], before["actions"]);
    assert_eq!(
        serde_json::to_value(session_history(&db, &sid).unwrap()).unwrap(),
        history_before
    );
    assert_eq!(first.actual_targets[0].ancestors[0].id, parent.id());
    let mut edit = input.clone();
    edit.summary.as_mut().unwrap().targets[0].observation = "合成修订观察".into();
    let revised = store::write_foundation_session(
        &mut db,
        &u,
        session::Write {
            id: sid.clone(),
            operation_id: id(),
            expected_version: Some(1),
            action: session::Action::Complete,
            input: Some(edit),
            change_note: "合成更正当时表现".into(),
        },
    )
    .unwrap();
    assert_eq!(revised.actual_targets[0].goal.id, a.id());
    assert_eq!(
        serde_json::to_value(&revised.actual_targets[0].ancestors).unwrap(),
        serde_json::to_value(&first.actual_targets[0].ancestors).unwrap()
    );
    assert_eq!(revised.actual_targets[0].input.note, "合成修订观察");
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            session::Write {
                id: id(),
                operation_id: id(),
                expected_version: None,
                action: session::Action::Complete,
                input: Some(input),
                change_note: "禁止新记录借用已删除目标".into()
            }
        )
        .is_err()
    );
    assert_eq!(session_history(&db, &sid).unwrap().len(), 2);
}

#[test]
fn retained_followup_bindings_are_editable_but_never_available_for_new_records() {
    let (mut db, u) = setup();
    let a = put(&mut db, &u, "合成待移除甲", Level::Short, None);
    put(&mut db, &u, "合成待移除乙", Level::Short, None);
    let kid = id();
    let command = json!({"kind":"knowledge","value":{"id":kid,"operation_id":id(),"expected_version":null,"action":"create","details":{"title":"合成保留困难"},"input":{"id":kid,"kind":"difficulty","content":"合成原困难","goal_ids":[a.id()]}}});
    store::foundation_process(
        &mut db,
        &u,
        serde_json::from_value(command.clone()).unwrap(),
    )
    .unwrap();
    let before = state(&mut db);
    assert_eq!(
        store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap()["completed"],
        true
    );
    let after = state(&mut db);
    assert_eq!(after["knowledge"], before["knowledge"]);
    assert_eq!(after["retired_goals"].as_array().unwrap().len(), 2);
    let mut edit = command.clone();
    edit["value"]["operation_id"] = json!(id());
    edit["value"]["expected_version"] = json!(1);
    edit["value"]["action"] = json!("edit");
    edit["value"]["input"]["content"] = json!("合成修订困难观察");
    let saved =
        store::foundation_process(&mut db, &u, serde_json::from_value(edit).unwrap()).unwrap();
    assert_eq!(saved["input"]["goal_ids"], json!([a.id()]));
    assert_eq!(saved["version"], 2);
    let other = id();
    let mut new = command;
    new["value"]["id"] = json!(other);
    new["value"]["input"]["id"] = json!(other);
    new["value"]["operation_id"] = json!(id());
    assert!(store::foundation_process(&mut db, &u, serde_json::from_value(new).unwrap()).is_err());
}

#[test]
fn old_metrics_measurements_and_moments_survive_but_new_deleted_goal_links_are_rejected() {
    let (mut db, u) = setup();
    let a = put(&mut db, &u, "合成待移除甲", Level::Short, None);
    put(&mut db, &u, "合成待移除乙", Level::Short, None);
    fn observe(
        db: &mut Connection,
        u: &User,
        id: &str,
        kind: &str,
        version: Option<u64>,
        input: Value,
    ) -> store::Result<Value> {
        store::foundation_process(db,u,serde_json::from_value(json!({"kind":"observe","value":{"id":id,"operation_id":super_id(),"expected_version":version,"action":"save","kind":kind,"note":"合成核对","input":input}})).unwrap())
    }
    fn super_id() -> String {
        id()
    }
    let metric = id();
    let metric_input = json!({"goal_id":a.id(),"goal_version":1,"name":"合成测量指标","template":"count","criterion":"5次机会","support":"示范一次","unit":""});
    observe(&mut db, &u, &metric, "metric", None, metric_input.clone()).unwrap();
    let measure = id();
    let measure_input = json!({"business_date":"2020-01-01","metric_id":metric,"metric_version":1,"opportunities":5,"achieved":2});
    observe(
        &mut db,
        &u,
        &measure,
        "measurement",
        None,
        measure_input.clone(),
    )
    .unwrap();
    let moment = id();
    let moment_input =
        json!({"business_date":"2020-01-01","text":"合成原瞬间","goal_ids":[a.id()]});
    observe(&mut db, &u, &moment, "moment", None, moment_input.clone()).unwrap();
    let rows_before: Vec<String> = db
        .prepare("SELECT data FROM prd_observations ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap()["completed"],
        true
    );
    let rows_after: Vec<String> = db
        .prepare("SELECT data FROM prd_observations ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows_after, rows_before);
    observe(
        &mut db,
        &u,
        &metric,
        "metric",
        Some(1),
        metric_input.clone(),
    )
    .unwrap();
    let mut edit = measure_input.clone();
    edit["achieved"] = json!(3);
    observe(&mut db, &u, &measure, "measurement", Some(1), edit).unwrap();
    let mut edit = moment_input.clone();
    edit["text"] = json!("合成修订瞬间");
    observe(&mut db, &u, &moment, "moment", Some(1), edit).unwrap();
    assert!(observe(&mut db, &u, &id(), "metric", None, metric_input).is_err());
    assert!(observe(&mut db, &u, &id(), "measurement", None, measure_input).is_err());
    assert!(observe(&mut db, &u, &id(), "moment", None, moment_input).is_err());
}
