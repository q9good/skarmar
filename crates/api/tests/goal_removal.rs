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
    for case in ["missing", "duplicate", "child", "long"] {
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
        let before = state(&mut db);
        assert_eq!(
            store::remove_selected_goals(&mut db, removal::TEST_ACK).unwrap()["completed"],
            false,
            "{case}"
        );
        assert_eq!(state(&mut db), before, "{case}");
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
