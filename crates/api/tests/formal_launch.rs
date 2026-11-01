//! One-time formal launch rehearsal. All accounts and records here are synthetic.
use rusqlite::{Connection, types::Value};
use serde_json::{Value as Json, json};
use skarma_api::store;
use skarma_core::{
    Command,
    auth::{AccountRole, AuthCommand, NewUser},
    foundation::{launch, model::*},
};
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn write(db: &mut Connection, actor: &skarma_core::auth::User, mutation: Mutation) -> Saved {
    store::write_foundation(
        db,
        actor,
        Write {
            id: id(),
            operation_id: id(),
            expected_version: None,
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            change_note: String::new(),
            mutation,
        },
    )
    .unwrap()
}
fn dump(db: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables:Vec<String>=db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |r|r.get(0)).unwrap().map(Result::unwrap).collect();
    tables
        .into_iter()
        .map(|name| {
            let mut stmt = db
                .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
                .unwrap();
            let cols = stmt.column_count();
            let rows = stmt
                .query_map([], |r| {
                    (0..cols)
                        .map(|n| r.get(n))
                        .collect::<rusqlite::Result<Vec<Value>>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            (name, rows)
        })
        .collect()
}
fn fixture() -> (Connection, String, Json) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let actor = store::bootstrap_admin(
        &mut db,
        NewUser {
            username: "synthetic-owner".into(),
            display_name: "合成家长".into(),
            password: "synthetic-pass".into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    store::seed_demo(&mut db).unwrap();
    store::initialize_empty_prd_baseline(&db).unwrap();
    let mut retained = Vec::new();
    for kind in [
        CatalogKind::Course,
        CatalogKind::Teacher,
        CatalogKind::Domain,
    ] {
        let result = write(
            &mut db,
            &actor,
            Mutation::Catalog(CatalogInput {
                kind,
                name: id(),
                active: true,
            }),
        );
        retained.push(serde_json::to_value(result).unwrap());
    }
    let Saved::Goal(goal) = write(
        &mut db,
        &actor,
        Mutation::Goal(
            serde_json::from_value(
                json!({"title":"合成长期方向","level":"long","long_priority":2}),
            )
            .unwrap(),
        ),
    ) else {
        panic!()
    };
    retained.push(json!({"kind":"goal","value":goal}));
    write(&mut db,&actor,Mutation::Goal(serde_json::from_value(json!({"title":"合成短期目标","level":"short","parent_id":goal.id,"criteria":"保留标准"})).unwrap()));
    write(
        &mut db,
        &actor,
        Mutation::Method(
            serde_json::from_value(json!({"title":"试用方法","goal_ids":[goal.id]})).unwrap(),
        ),
    );
    let login = store::execute(
        &mut db,
        Command::Auth(AuthCommand::Login {
            username: "synthetic-owner".into(),
            password: "synthetic-pass".into(),
        }),
    )
    .unwrap();
    let token = login["token"].as_str().unwrap().to_owned();
    db.execute_batch("INSERT INTO prd_operations VALUES('trial-op','trial-actor','{}','{}');
        INSERT INTO prd_operations VALUES('trial-op2','trial-actor','{}','{}');
        INSERT INTO prd_lessons VALUES('trial-lesson',1,'{}');
        INSERT INTO prd_sessions VALUES('trial-session',1,'{}');
        INSERT INTO prd_lesson_revisions VALUES('trial-lesson',1,'trial-op','{}');
        INSERT INTO prd_session_revisions VALUES('trial-session',1,'trial-op2','{}');
        INSERT INTO prd_lesson_refs VALUES('trial-lesson','goal','trial-reference');
        INSERT INTO prd_reviews VALUES('trial-review','goal','trial-reference',1,'2026-11-01',1,'{}');
        INSERT INTO prd_review_revisions VALUES('trial-review',1,'{}');
        INSERT INTO prd_feedback VALUES('trial-feedback','trial-knowledge','independent','trial-source',1,'{}');
        INSERT INTO prd_feedback_revisions VALUES('trial-feedback',1,'{}');
        INSERT INTO prd_annotations VALUES('trial-annotation','trial-feedback','{}');
        INSERT INTO prd_knowledge(id,source_lesson_id,data) VALUES('trial-knowledge','trial-lesson','{}');
        INSERT INTO prd_actions(id,source_session_id,data) VALUES('trial-action','trial-session','{}');
        INSERT INTO prd_output_revisions VALUES('knowledge','trial-knowledge',1,'{}');
        INSERT INTO prd_observations VALUES('trial-observation','moment','trial-key',1,'{}');
        INSERT INTO prd_observation_revisions VALUES('trial-observation',1,'{}');
        INSERT INTO prd_import_previews VALUES('trial-preview','{}');
        INSERT INTO prd_import_rows VALUES('trial-row','{}');
        INSERT INTO prd_import_batches VALUES('trial-batch','{}');
        INSERT INTO prd_schedules VALUES('knowledge','trial-knowledge',1,'{}');
        INSERT INTO prd_schedule_revisions VALUES('knowledge','trial-knowledge',1,'{}');
        INSERT INTO prd_upgrade VALUES('trial-import-marker','{}');").unwrap();
    (db, token, json!(retained))
}

#[test]
fn trial_cleanup_preserves_goals_catalog_accounts_and_never_repeats_after_new_writes() {
    let (mut db, token, _) = fixture();
    let before = dump(&db);
    let goal_revisions:Vec<String>=db.prepare("SELECT data FROM prd_revisions WHERE entity_id IN (SELECT id FROM prd_entities WHERE kind<>'method') ORDER BY entity_id,version").unwrap().query_map([],|r|r.get(0)).unwrap().map(Result::unwrap).collect();
    let result = store::formal_launch(&mut db, launch::ACK).unwrap();
    assert_eq!(result, json!({"epoch":launch::EPOCH,"completed":true}));
    let after = dump(&db);
    for name in [
        "auth_users",
        "auth_sessions",
        "auth_login_attempts",
        "auth_registration_requests",
        "auth_registration_limits",
        "goals",
        "goal_revisions",
        "lesson_options",
        "schema_migrations",
        "prd_settings",
    ] {
        assert_eq!(
            before.iter().find(|(n, _)| n == name),
            after.iter().find(|(n, _)| n == name),
            "preserve {name}"
        );
    }
    let after_revisions: Vec<String> = db
        .prepare("SELECT data FROM prd_revisions ORDER BY entity_id,version")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(goal_revisions, after_revisions);
    let snapshot = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(snapshot.goals.len(), 2);
    assert_eq!(snapshot.catalog.len(), 3);
    assert!(
        snapshot.methods.is_empty()
            && snapshot.lessons.is_empty()
            && snapshot.sessions.is_empty()
            && snapshot.knowledge.is_empty()
            && snapshot.actions.is_empty()
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
    for epoch in [None, Some("trial".into())] {
        let error = store::execute(
            &mut db,
            Command::Authenticated {
                token: token.clone(),
                expected_actor: None,
                data_epoch: epoch,
                command: Box::new(Command::FoundationState),
            },
        )
        .unwrap_err();
        assert_eq!(error.1, launch::STALE_PAGE);
    }
    assert!(
        store::execute(
            &mut db,
            Command::Authenticated {
                token: token.clone(),
                expected_actor: None,
                data_epoch: Some(launch::EPOCH.into()),
                command: Box::new(Command::FoundationState)
            }
        )
        .is_ok()
    );
    // New formal data survives both repeated launch hook and normal initialization.
    db.execute(
        "INSERT INTO prd_import_batches VALUES('formal-new-batch','{}')",
        [],
    )
    .unwrap();
    store::initialize_empty_prd_baseline(&db).unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    let formal = dump(&db);
    store::formal_launch(&mut db, launch::ACK).unwrap();
    assert_eq!(dump(&db), formal);
    let login = store::execute(
        &mut db,
        Command::Auth(AuthCommand::Login {
            username: "synthetic-owner".into(),
            password: "synthetic-pass".into(),
        }),
    )
    .unwrap();
    assert!(login["token"].is_string());
}

#[test]
fn failure_mid_cleanup_rolls_back_every_trial_row_and_marker() {
    let (mut db, _, _) = fixture();
    db.execute_batch("CREATE TRIGGER fail_cleanup BEFORE DELETE ON prd_observations BEGIN SELECT RAISE(ABORT,'synthetic refusal'); END;").unwrap();
    let before = dump(&db);
    assert!(store::formal_launch(&mut db, launch::ACK).is_err());
    assert_eq!(dump(&db), before);
}

#[test]
fn refuses_wrong_confirmation_unknown_tables_missing_admin_and_marker_corruption() {
    let (mut db, _, _) = fixture();
    let before = dump(&db);
    assert!(store::formal_launch(&mut db, "not-the-confirmation").is_err());
    assert_eq!(dump(&db), before);
    db.execute_batch("CREATE TABLE unknown_trial(data TEXT);")
        .unwrap();
    let before = dump(&db);
    assert!(store::formal_launch(&mut db, launch::ACK).is_err());
    assert_eq!(dump(&db), before);
    db.execute_batch("DROP TABLE unknown_trial;UPDATE auth_users SET active=0;")
        .unwrap();
    let before = dump(&db);
    assert!(store::formal_launch(&mut db, launch::ACK).is_err());
    assert_eq!(dump(&db), before);
    db.execute_batch("UPDATE auth_users SET active=1;").unwrap();
    store::formal_launch(&mut db, launch::ACK).unwrap();
    db.execute(
        "UPDATE prd_upgrade SET data='{}' WHERE id='formal-launch-20261101'",
        [],
    )
    .unwrap();
    let before = dump(&db);
    assert!(store::formal_launch(&mut db, launch::ACK).is_err());
    assert_eq!(dump(&db), before);
    let mut legacy = Connection::open_in_memory().unwrap();
    store::initialize(&legacy).unwrap();
    let before = dump(&legacy);
    assert!(store::formal_launch(&mut legacy, launch::ACK).is_err());
    assert_eq!(dump(&legacy), before);
}

#[test]
fn unexpected_preserved_data_change_or_ignored_trial_delete_rolls_back() {
    for trigger in [
        "CREATE TRIGGER tamper_preserved AFTER DELETE ON prd_observations BEGIN UPDATE auth_users SET display_name='unexpected'; END;",
        "CREATE TRIGGER ignore_method BEFORE DELETE ON prd_entities WHEN OLD.kind='method' BEGIN SELECT RAISE(IGNORE); END;",
    ] {
        let (mut db, _, _) = fixture();
        db.execute_batch(trigger).unwrap();
        let before = dump(&db);
        assert!(store::formal_launch(&mut db, launch::ACK).is_err());
        assert_eq!(dump(&db), before);
    }
}
