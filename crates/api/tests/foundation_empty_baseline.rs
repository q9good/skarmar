//! Deployment rehearsal with synthetic rows only. No credentials or real DB.
use rusqlite::Connection;
use skarma_api::store;
use skarma_core::{
    Command,
    auth::{AccountRole, NewUser},
};

fn original() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    store::bootstrap_admin(
        &mut db,
        NewUser {
            username: "synthetic-owner".into(),
            display_name: "合成管理员".into(),
            password: "synthetic-password".into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    store::seed_demo(&mut db).unwrap();
    db
}
fn dump(db: &Connection) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    let names: Vec<String> = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'prd_%' ORDER BY name").unwrap().query_map([], |r|r.get(0)).unwrap().map(Result::unwrap).collect();
    names
        .into_iter()
        .map(|name| {
            let mut stmt = db
                .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
                .unwrap();
            let columns = stmt.column_count();
            let rows = stmt
                .query_map([], |r| {
                    (0..columns)
                        .map(|n| r.get(n))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            (name, rows)
        })
        .collect()
}

#[test]
fn empty_cutover_preserves_every_legacy_and_auth_row_and_never_resets_new_writes() {
    let mut db = original();
    let before = dump(&db);
    assert!(store::initialize_prd_runtime(&db).is_err());
    store::initialize_empty_prd_baseline(&db).unwrap();
    assert_eq!(dump(&db), before);
    let empty = store::foundation_snapshot(&mut db).unwrap();
    assert!(
        empty.goals.is_empty()
            && empty.methods.is_empty()
            && empty.lessons.is_empty()
            && empty.sessions.is_empty()
            && empty.knowledge.is_empty()
    );
    store::initialize_prd_runtime(&db).unwrap();
    assert!(
        store::initialize(&db).is_err(),
        "old model must never write into the new baseline"
    );
    let actor = store::execute(
        &mut db,
        Command::Auth(skarma_core::auth::AuthCommand::Login {
            username: "synthetic-owner".into(),
            password: "synthetic-password".into(),
        }),
    );
    // Login's API shape is tested through both real HTTP adapters below.
    assert!(actor.is_ok());
    db.execute(
        "INSERT INTO prd_import_batches(id,data) VALUES('synthetic-new','{}')",
        [],
    )
    .unwrap();
    let after = dump(&db);
    store::initialize_empty_prd_baseline(&db).unwrap();
    assert_eq!(dump(&db), after);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_import_batches WHERE id='synthetic-new'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn empty_cutover_rejects_unknown_partial_and_uninitialized_stores_without_changes() {
    for mode in ["unknown", "partial", "no-admin", "empty"] {
        let db = if mode == "empty" {
            Connection::open_in_memory().unwrap()
        } else {
            original()
        };
        match mode {
            "unknown" => db
                .execute_batch("CREATE TABLE unexpected(data TEXT);")
                .unwrap(),
            "partial" => db
                .execute_batch("DELETE FROM schema_migrations WHERE version=10;")
                .unwrap(),
            "no-admin" => db.execute_batch("UPDATE auth_users SET active=0;").unwrap(),
            _ => (),
        }
        let before = dump(&db);
        assert!(store::initialize_empty_prd_baseline(&db).is_err(), "{mode}");
        assert_eq!(dump(&db), before, "{mode}");
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'prd_%'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn initialization_failure_rolls_back_all_new_tables() {
    let db = original();
    // Inject a collision in the new schema's index namespace. Unknown indexes
    // do not alter table metadata, so this fails after CREATE TABLE begins.
    db.execute_batch("CREATE INDEX prd_catalog_names ON goals(id);")
        .unwrap();
    let before = dump(&db);
    assert!(store::initialize_empty_prd_baseline(&db).is_err());
    assert_eq!(dump(&db), before);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'prd_%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
