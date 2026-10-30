//! Synthetic tests of the explicit PRD model entry; never touch the application database.
use axum::http::StatusCode;
use rusqlite::Connection;
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::model::*,
};
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn actor() -> User {
    User {
        id: id(),
        username: "synthetic-member".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    }
}
fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    db
}
fn goal(title: &str, level: &str, parent: Option<String>) -> Mutation {
    Mutation::Goal(
        serde_json::from_value(serde_json::json!({"title":title,"level":level,"parent_id":parent}))
            .unwrap(),
    )
}
fn write(identifier: &str, expected_version: Option<u64>, mutation: Mutation) -> Write {
    Write {
        provided_fields: None,
        method_change: None,
        method_create_mode: None,
        id: identifier.into(),
        operation_id: id(),
        expected_version,
        change_note: String::new(),
        mutation,
    }
}
fn method(goal_ids: Vec<String>) -> Mutation {
    Mutation::Method(
        serde_json::from_value(serde_json::json!({"title":"测试方法","goal_ids":goal_ids}))
            .unwrap(),
    )
}
fn catalog(name: &str, active: bool) -> Mutation {
    Mutation::Catalog(CatalogInput {
        kind: CatalogKind::Teacher,
        name: name.into(),
        active,
    })
}

#[test]
fn refuses_legacy_database_and_never_changes_its_tables() {
    let db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let before: i64 = db
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        store::initialize_foundation(&db).unwrap_err().0,
        StatusCode::CONFLICT
    );
    let after: i64 = db
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'prd_%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let isolated = database();
    store::initialize_foundation(&isolated).unwrap();
}

#[test]
fn state_lifecycle_has_history_and_distinguishes_archive_from_achievement() {
    let mut db = database();
    let actor = actor();
    let identifier = id();
    let initial = store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, None, goal("测试近期本领", "short", None)),
    )
    .unwrap();
    let Saved::Goal(mut g) = initial else {
        panic!()
    };
    assert_eq!(g.input.state, Some(GoalState::Active));
    assert_eq!(g.input.frequency, None);
    g.input.frequency = Some(Frequency::None);
    g.input.archived = true;
    let mut update = write(&identifier, Some(1), Mutation::Goal(g.input.clone()));
    assert_eq!(
        store::write_foundation(&mut db, &actor, update.clone())
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    update.change_note = "暂时收起，不代表达成".into();
    store::write_foundation(&mut db, &actor, update).unwrap();
    let history = store::foundation_history(&mut db, &identifier).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].event, "state_changed");
    let Saved::Goal(old) = &history[0].entity else {
        panic!()
    };
    let Saved::Goal(new) = &history[1].entity else {
        panic!()
    };
    assert!(!old.input.archived);
    assert!(new.input.archived);
    assert_eq!(new.input.state, Some(GoalState::Active));
    assert_eq!(new.input.frequency, Some(Frequency::None));
    assert_eq!(history[1].actor_id, actor.id);
    let long_id = id();
    let initial = store::write_foundation(
        &mut db,
        &actor,
        write(&long_id, None, goal("测试长期方向", "long", None)),
    )
    .unwrap();
    let Saved::Goal(mut long) = initial else {
        panic!()
    };
    long.input.state = Some(GoalState::Achieved);
    assert_eq!(
        store::write_foundation(
            &mut db,
            &actor,
            write(&long_id, Some(1), Mutation::Goal(long.input))
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[test]
fn goal_parent_constraints_and_missing_method_links_are_atomic() {
    let mut db = database();
    let actor = actor();
    let long = id();
    let medium = id();
    let short = id();
    store::write_foundation(
        &mut db,
        &actor,
        write(&long, None, goal("测试长期", "long", None)),
    )
    .unwrap();
    store::write_foundation(
        &mut db,
        &actor,
        write(
            &medium,
            None,
            goal("测试中期", "medium", Some(long.clone())),
        ),
    )
    .unwrap();
    store::write_foundation(
        &mut db,
        &actor,
        write(
            &short,
            None,
            goal("测试短期", "short", Some(medium.clone())),
        ),
    )
    .unwrap();
    let Saved::Goal(mut parent) = store::foundation_snapshot(&mut db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.id == medium)
        .map(Saved::Goal)
        .unwrap()
    else {
        panic!()
    };
    parent.input.parent_id = Some(short.clone());
    assert_eq!(
        store::write_foundation(
            &mut db,
            &actor,
            write(&medium, Some(1), Mutation::Goal(parent.input))
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let invalid = write(&id(), None, method(vec![short.clone(), id()]));
    assert_eq!(
        store::write_foundation(&mut db, &actor, invalid)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(
        store::foundation_snapshot(&mut db)
            .unwrap()
            .methods
            .is_empty()
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_method_goals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_operations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn retry_is_idempotent_but_changed_payload_other_actor_or_stale_version_conflicts() {
    let mut db = database();
    let actor = actor();
    let identifier = id();
    let request = write(&identifier, None, goal("测试原文", "short", None));
    let created = store::write_foundation(&mut db, &actor, request.clone()).unwrap();
    assert_eq!(
        created,
        store::write_foundation(&mut db, &actor, request.clone()).unwrap()
    );
    let mut changed = request.clone();
    changed.change_note = "不是同一次提交".into();
    assert_eq!(
        store::write_foundation(&mut db, &actor, changed)
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    let other = User {
        id: id(),
        ..actor.clone()
    };
    assert_eq!(
        store::write_foundation(&mut db, &other, request)
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, Some(1), goal("测试修订", "short", None)),
    )
    .unwrap();
    assert_eq!(
        store::write_foundation(
            &mut db,
            &actor,
            write(&identifier, Some(1), goal("过期改动", "short", None))
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store::foundation_history(&mut db, &identifier)
            .unwrap()
            .len(),
        2
    );
    let inactive = User {
        active: false,
        ..actor
    };
    assert_eq!(
        store::write_foundation(
            &mut db,
            &inactive,
            write(&id(), None, goal("停用用户", "short", None))
        )
        .unwrap_err()
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn method_versions_keep_original_goal_links_and_name_only_creation_is_supported() {
    let mut db = database();
    let actor = actor();
    let a = id();
    let b = id();
    let m = id();
    for identifier in [&a, &b] {
        store::write_foundation(
            &mut db,
            &actor,
            write(identifier, None, goal("测试本领", "short", None)),
        )
        .unwrap();
    }
    let first =
        store::write_foundation(&mut db, &actor, write(&m, None, method(vec![a.clone()]))).unwrap();
    let Saved::Method(mut record) = first.clone() else {
        panic!()
    };
    assert!(record.input.description.is_empty());
    record.input.goal_ids = vec![a.clone(), b.clone()];
    record.input.description = "修订后的做法".into();
    store::write_foundation(
        &mut db,
        &actor,
        write(&m, Some(1), Mutation::Method(record.input)),
    )
    .unwrap();
    let history = store::foundation_history(&mut db, &m).unwrap();
    assert_eq!(history[0].entity, first);
    assert_eq!(history[0].entity.version(), 1);
    assert_eq!(history[1].entity.version(), 2);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_method_goals WHERE method_id=?1",
            [&m],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    let duplicate = method(vec![a.clone(), a]);
    assert_eq!(
        store::write_foundation(&mut db, &actor, write(&id(), None, duplicate))
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[test]
fn catalog_rename_keeps_id_and_old_snapshots_while_stop_restore_preserves_history() {
    let mut db = database();
    let actor = actor();
    let identifier = id();
    store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, None, catalog("测试老师旧名", true)),
    )
    .unwrap();
    let old =
        store::foundation_catalog_snapshot(&mut db, &identifier, CatalogKind::Teacher).unwrap();
    store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, Some(1), catalog("测试老师新名", true)),
    )
    .unwrap();
    let current =
        store::foundation_catalog_snapshot(&mut db, &identifier, CatalogKind::Teacher).unwrap();
    assert_eq!(old.id, current.id);
    assert_eq!(old.name, "测试老师旧名");
    assert_eq!(current.name, "测试老师新名");
    assert_eq!(old.version, 1);
    assert_eq!(current.version, 2);
    assert_eq!(
        store::write_foundation(
            &mut db,
            &actor,
            write(&id(), None, catalog("测试老师新名", true))
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, Some(2), catalog("测试老师新名", false)),
    )
    .unwrap();
    assert_eq!(
        store::foundation_catalog_snapshot(&mut db, &identifier, CatalogKind::Teacher)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        store::write_foundation(
            &mut db,
            &actor,
            write(&id(), None, catalog("测试老师新名", true))
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    store::write_foundation(
        &mut db,
        &actor,
        write(&identifier, Some(3), catalog("测试老师新名", true)),
    )
    .unwrap();
    assert_eq!(
        store::foundation_catalog_snapshot(&mut db, &identifier, CatalogKind::Teacher)
            .unwrap()
            .id,
        identifier
    );
    assert_eq!(
        store::foundation_history(&mut db, &identifier)
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn method_creation_modes_require_full_steps_but_allow_context_quick_name_and_goals() {
    let mut db = database();
    let u = actor();
    let goal_id = id();
    store::write_foundation(
        &mut db,
        &u,
        write(&goal_id, None, goal("测试目标", "short", None)),
    )
    .unwrap();
    let command =
        |identifier: &str, mode: Option<&str>, version: Option<u64>, value: serde_json::Value| {
            let mut raw = serde_json::json!({
                "id":identifier,
                "operation_id":id(),
                "expected_version":version,
                "change_note":"测试创建模式",
                "mutation":{"kind":"method","value":value}
            });
            if let Some(mode) = mode {
                raw["method_create_mode"] = serde_json::json!(mode);
            }
            serde_json::from_value::<Write>(raw).unwrap()
        };

    let missing = id();
    assert_eq!(
        store::write_foundation(
            &mut db,
            &u,
            command(
                &missing,
                None,
                None,
                serde_json::json!({"title":"未标模式","goal_ids":[goal_id]})
            )
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let full = id();
    assert_eq!(
        store::write_foundation(
            &mut db,
            &u,
            command(
                &full,
                Some("full"),
                None,
                serde_json::json!({"title":"普通方法","goal_ids":[goal_id]})
            )
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let full_saved = store::write_foundation(
        &mut db,
        &u,
        command(
            &full,
            Some("full"),
            None,
            serde_json::json!({
                "title":"普通方法",
                "steps":"先跟着他的玩法",
                "observe":"愿意再试一次",
                "scope":"熟悉的游戏中",
                "goal_ids":[goal_id]
            }),
        ),
    )
    .unwrap();
    let Saved::Method(full_saved) = full_saved else {
        panic!()
    };
    assert_eq!(full_saved.input.steps, "先跟着他的玩法");
    assert_eq!(full_saved.input.observe, "愿意再试一次");
    assert_eq!(full_saved.input.scope, "熟悉的游戏中");
    assert_eq!(store::foundation_history(&mut db, &full).unwrap().len(), 1);

    let quick = id();
    let quick_request = command(
        &quick,
        Some("quick"),
        None,
        serde_json::json!({"title":"快捷方法","goal_ids":[goal_id]}),
    );
    let quick_saved = store::write_foundation(&mut db, &u, quick_request.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(store::write_foundation(&mut db, &u, quick_request).unwrap()).unwrap(),
        serde_json::to_value(&quick_saved).unwrap()
    );
    let Saved::Method(quick_saved) = quick_saved else {
        panic!()
    };
    assert!(quick_saved.input.steps.is_empty());
    assert_eq!(quick_saved.input.goal_ids, vec![goal_id.clone()]);
    assert_eq!(store::foundation_history(&mut db, &quick).unwrap().len(), 1);

    assert_eq!(
        store::write_foundation(
            &mut db,
            &u,
            command(
                &quick,
                Some("full"),
                Some(1),
                serde_json::json!({
                    "title":"快捷方法补充",
                    "steps":"后来补充具体做法",
                    "goal_ids":[goal_id]
                })
            )
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(
        serde_json::from_value::<Write>(serde_json::json!({
            "id":id(),
            "operation_id":id(),
            "expected_version":null,
            "change_note":"",
            "method_create_mode":"import",
            "mutation":{"kind":"method","value":{"title":"非法模式","goal_ids":[goal_id]}}
        }))
        .is_err()
    );
}

#[test]
fn goal_domain_due_and_support_are_optional_versioned_and_safe_for_legacy_records() {
    let mut db = database();
    let u = actor();
    let domain_id = id();
    let saved_domain = store::write_foundation(
        &mut db,
        &u,
        write(
            &domain_id,
            None,
            Mutation::Catalog(CatalogInput {
                kind: CatalogKind::Domain,
                name: "沟通互动".into(),
                active: true,
            }),
        ),
    )
    .unwrap();
    let Saved::Catalog(domain) = saved_domain else {
        panic!()
    };
    let domain = CatalogSnapshot {
        id: domain.id,
        version: domain.version,
        kind: CatalogKind::Domain,
        name: domain.input.name,
    };
    let goal_id = id();
    let command = |version: Option<u64>, value: serde_json::Value| {
        serde_json::from_value::<Write>(serde_json::json!({
            "id":goal_id,
            "operation_id":id(),
            "expected_version":version,
            "change_note":"合成字段核对",
            "mutation":{"kind":"goal","value":value}
        }))
        .unwrap()
    };
    let saved = store::write_foundation(
        &mut db,
        &u,
        command(
            None,
            serde_json::json!({
                "title":"主动求助",
                "level":"short",
                "criteria":"需要帮助时主动表达",
                "support":"先不提示，需要时再示范",
                "domain":domain,
                "due":"2027-01-20"
            }),
        ),
    )
    .unwrap();
    let Saved::Goal(goal) = saved else { panic!() };
    assert_eq!(goal.input.domain.as_ref().unwrap().name, "沟通互动");
    assert_eq!(goal.input.due.as_deref(), Some("2027-01-20"));
    assert_eq!(goal.input.support, "先不提示，需要时再示范");

    let renamed = store::write_foundation(
        &mut db,
        &u,
        write(
            &domain_id,
            Some(1),
            Mutation::Catalog(CatalogInput {
                kind: CatalogKind::Domain,
                name: "表达与互动".into(),
                active: true,
            }),
        ),
    )
    .unwrap();
    assert_eq!(renamed.version(), 2);
    let partial = store::write_foundation(
        &mut db,
        &u,
        command(
            Some(1),
            serde_json::json!({"title":"主动表达需要","level":"short"}),
        ),
    )
    .unwrap();
    let Saved::Goal(partial) = partial else {
        panic!()
    };
    assert_eq!(partial.input.domain.as_ref().unwrap().name, "沟通互动");
    assert_eq!(partial.input.domain.as_ref().unwrap().version, 1);
    assert_eq!(partial.input.due.as_deref(), Some("2027-01-20"));
    assert_eq!(partial.input.support, "先不提示，需要时再示范");

    let cleared = store::write_foundation(
        &mut db,
        &u,
        command(
            Some(2),
            serde_json::json!({
                "title":"主动表达需要",
                "level":"short",
                "domain":null,
                "due":null,
                "support":""
            }),
        ),
    )
    .unwrap();
    let Saved::Goal(cleared) = cleared else {
        panic!()
    };
    assert!(cleared.input.domain.is_none());
    assert!(cleared.input.due.is_none());
    assert!(cleared.input.support.is_empty());
    assert_eq!(
        store::write_foundation(
            &mut db,
            &u,
            command(
                Some(3),
                serde_json::json!({
                    "title":"主动表达需要",
                    "level":"short",
                    "due":"2027-02-31"
                }),
            ),
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let legacy: Saved = serde_json::from_value(serde_json::json!({
        "kind":"goal",
        "value":{
            "id":id(),
            "version":1,
            "input":{"title":"旧目标","level":"long"}
        }
    }))
    .unwrap();
    let Saved::Goal(legacy) = legacy else {
        panic!()
    };
    assert!(legacy.input.domain.is_none());
    assert!(legacy.input.due.is_none());
    assert!(legacy.input.support.is_empty());
}

#[test]
fn failing_history_write_rolls_back_entity_operation_and_relationships() {
    let mut db = database();
    let actor = actor();
    let g = id();
    store::write_foundation(
        &mut db,
        &actor,
        write(&g, None, goal("测试本领", "short", None)),
    )
    .unwrap();
    db.execute_batch("CREATE TRIGGER fail_revision BEFORE INSERT ON prd_revisions BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let request = write(&id(), None, method(vec![g]));
    assert_eq!(
        store::write_foundation(&mut db, &actor, request.clone())
            .unwrap_err()
            .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        store::foundation_snapshot(&mut db)
            .unwrap()
            .methods
            .is_empty()
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_method_goals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_operations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    db.execute_batch("DROP TRIGGER fail_revision").unwrap();
    assert_eq!(
        store::write_foundation(&mut db, &actor, request)
            .unwrap()
            .version(),
        1
    );
}

#[test]
fn two_connections_cannot_overwrite_the_same_version() {
    use std::{
        sync::{Arc, Barrier},
        time::Duration,
    };
    let path = std::env::temp_dir().join(format!("skarmar-foundation-{}.sqlite", id()));
    let identifier = id();
    let mut db = Connection::open(&path).unwrap();
    store::initialize_foundation(&db).unwrap();
    store::write_foundation(
        &mut db,
        &actor(),
        write(&identifier, None, goal("测试原始目标", "short", None)),
    )
    .unwrap();
    drop(db);
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = vec![];
    for title in ["测试妈妈修改", "测试爸爸修改"] {
        let path = path.clone();
        let barrier = barrier.clone();
        let identifier = identifier.clone();
        threads.push(std::thread::spawn(move || {
            let mut db = Connection::open(path).unwrap();
            db.busy_timeout(Duration::from_secs(5)).unwrap();
            store::initialize_foundation(&db).unwrap();
            barrier.wait();
            store::write_foundation(
                &mut db,
                &actor(),
                write(&identifier, Some(1), goal(title, "short", None)),
            )
        }));
    }
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r,Err(error) if error.0==StatusCode::CONFLICT))
            .count(),
        1
    );
    let mut db = Connection::open(&path).unwrap();
    let history = store::foundation_history(&mut db, &identifier).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].entity.version(), 2);
    assert_ne!(history[0].actor_id, history[1].actor_id);
    drop(db);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn authenticated_initializer_preserves_users_and_refuses_legacy_reinitialization() {
    use skarma_core::{Command, auth::NewUser};
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    let user = store::bootstrap_admin(
        &mut db,
        NewUser {
            username: "synthetic-owner".into(),
            display_name: "测试管理员".into(),
            password: "s".into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    let persisted: String = db
        .query_row(
            "SELECT id FROM auth_users WHERE username='synthetic-owner'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(persisted, user.id);
    let before: i64 = db
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert_eq!(store::initialize(&db).unwrap_err().0, StatusCode::CONFLICT);
    let after: i64 = db
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        store::execute(&mut db, Command::State).unwrap_err().0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store::execute(&mut db, Command::FoundationState)
            .unwrap_err()
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        store::execute(&mut db, Command::RuntimeInfo).unwrap()["model"],
        "prd_v1"
    );
    let mut legacy = Connection::open_in_memory().unwrap();
    store::initialize(&legacy).unwrap();
    assert_eq!(
        store::execute(&mut legacy, Command::FoundationState)
            .unwrap_err()
            .0,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn omitted_update_fields_preserve_existing_values_but_explicit_empty_and_null_clear_them() {
    let mut db = database();
    let u = actor();
    let g = id();
    let mid = id();
    let command = |id: &str, v: Option<u64>, kind: &str, value: serde_json::Value| {
        serde_json::from_value::<Write>(serde_json::json!({"id":id,"operation_id":self::id(),"expected_version":v,"change_note":"合成修改说明","mutation":{"kind":kind,"value":value}})).unwrap()
    };
    store::write_foundation(&mut db,&u,command(&g,None,"goal",serde_json::json!({"title":"原目标","level":"short","meaning":"要保留的生活意义","criteria":"要保留的观察标准","state":"paused","resume_on":"2099-01-01","frequency":"monthly","archived":true}))).unwrap();
    let partial = command(
        &g,
        Some(1),
        "goal",
        serde_json::json!({"title":"只改目标名","level":"short"}),
    );
    let renamed = store::write_foundation(&mut db, &u, partial.clone()).unwrap();
    if let Saved::Goal(g) = &renamed {
        assert_eq!(g.input.meaning, "要保留的生活意义");
        assert_eq!(g.input.criteria, "要保留的观察标准");
        assert_eq!(g.input.resume_on.as_deref(), Some("2099-01-01"));
        assert_eq!(g.input.frequency, Some(Frequency::Monthly));
        assert!(g.input.archived);
    } else {
        panic!()
    }
    assert_eq!(
        serde_json::to_value(store::write_foundation(&mut db, &u, partial).unwrap()).unwrap(),
        serde_json::to_value(renamed).unwrap()
    );
    let cleared=store::write_foundation(&mut db,&u,command(&g,Some(2),"goal",serde_json::json!({"title":"明确清空","level":"short","meaning":"","criteria":"","resume_on":null,"archived":false}))).unwrap();
    if let Saved::Goal(g) = cleared {
        assert!(g.input.meaning.is_empty());
        assert!(g.input.criteria.is_empty());
        assert_eq!(g.input.resume_on, None);
        assert!(!g.input.archived);
        assert_eq!(g.input.frequency, Some(Frequency::Monthly));
    } else {
        panic!()
    }
    let mut method_create = command(
        &mid,
        None,
        "method",
        serde_json::json!({"title":"原方法","goal_ids":[g],"description":"要保留的做法","steps":"要保留的步骤","guidance":"老师示范","state":"paused","resume_on":"2099-01-01","pinned":true}),
    );
    method_create.method_create_mode = Some(MethodCreateMode::Full);
    store::write_foundation(&mut db, &u, method_create).unwrap();
    let edited = store::write_foundation(
        &mut db,
        &u,
        command(
            &mid,
            Some(1),
            "method",
            serde_json::json!({"title":"只改方法名","goal_ids":[g]}),
        ),
    )
    .unwrap();
    if let Saved::Method(m) = edited {
        assert_eq!(m.input.description, "要保留的做法");
        assert_eq!(m.input.steps, "要保留的步骤");
        assert_eq!(m.input.guidance, "老师示范");
        assert_eq!(m.input.resume_on.as_deref(), Some("2099-01-01"));
        assert!(m.input.pinned);
    } else {
        panic!()
    }
}

#[test]
fn field_presence_survives_transport_distinguishes_reused_operations_and_replays_legacy_receipts() {
    let mut db = database();
    let u = actor();
    let gid = id();
    let raw = |v: Option<u64>, value: serde_json::Value| serde_json::json!({"id":gid,"operation_id":id(),"expected_version":v,"change_note":"合成字段核对","mutation":{"kind":"goal","value":value}});
    store::write_foundation(&mut db,&u,serde_json::from_value(raw(None,serde_json::json!({"title":"原目标","level":"short","meaning":"不要清空","resume_on":"2099-01-01","state":"paused"}))).unwrap()).unwrap();
    let original = raw(
        Some(1),
        serde_json::json!({"title":"仅更名","level":"short"}),
    );
    let parsed: Write = serde_json::from_value(original.clone()).unwrap();
    let transported = serde_json::to_value(&parsed).unwrap();
    assert_eq!(
        transported["mutation"]["value"],
        original["mutation"]["value"]
    );
    assert!(transported.get("provided_fields").is_none());
    let wire: Write = serde_json::from_value(transported).unwrap();
    let legacy = wire.legacy_json().unwrap();
    let saved = store::write_foundation(&mut db, &u, wire.clone()).unwrap();
    if let Saved::Goal(g) = &saved {
        assert_eq!(g.input.meaning, "不要清空");
        assert_eq!(g.input.resume_on.as_deref(), Some("2099-01-01"));
    } else {
        panic!()
    }
    let mut changed = original.clone();
    changed["mutation"]["value"]["meaning"] = serde_json::json!("");
    assert_eq!(
        store::write_foundation(&mut db, &u, serde_json::from_value(changed).unwrap())
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        serde_json::to_value(store::write_foundation(&mut db, &u, wire.clone()).unwrap()).unwrap(),
        serde_json::to_value(&saved).unwrap()
    );
    // Existing operations from before field masks remain safely replayable.
    db.execute(
        "UPDATE prd_operations SET request=?2 WHERE id=?1",
        [&wire.operation_id, &legacy],
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(store::write_foundation(&mut db, &u, wire).unwrap()).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    let mut invalid = original;
    invalid["provided_fields"] = serde_json::json!(["meaning"]);
    assert!(serde_json::from_value::<Write>(invalid).is_err());
}

#[test]
fn added_optional_fields_preserve_prior_pending_request_serialization() {
    // These literal commands predate the priority/helpful-observation additions.
    // Empty additions must not enter the canonical command used as the saved
    // operation identity: a queued old request must still retrieve its receipt.
    let goal: GoalInput = serde_json::from_value(serde_json::json!({
        "title": "合成旧待确认目标", "level": "long"
    }))
    .unwrap();
    assert!(
        !serde_json::to_value(&goal)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("long_priority")
    );
    let observation: skarma_core::foundation::lesson_model::ObservationInput =
        serde_json::from_value(serde_json::json!({
            "business_date": "2026-10-01", "course_id": id(), "teacher_id": id()
        }))
        .unwrap();
    assert!(
        !serde_json::to_value(&observation)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("helpful_notes")
    );
    let mut db = database();
    let member = actor();
    let identifier = id();
    let command = write(&identifier, None, Mutation::Goal(goal));
    let receipt = store::write_foundation(&mut db, &member, command.clone()).unwrap();
    let encoded: String = db
        .query_row(
            "SELECT request FROM prd_operations WHERE id=?1",
            [&command.operation_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!encoded.contains("long_priority"));
    // Exact old requests replay, but reusing their operation ID for a new
    // priority is a conflict, not a silently accepted configuration change.
    let replay = store::write_foundation(&mut db, &member, command.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    let mut changed = command;
    if let Mutation::Goal(value) = &mut changed.mutation {
        value.long_priority = Some(2);
    }
    assert_eq!(
        store::write_foundation(&mut db, &member, changed)
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
}
