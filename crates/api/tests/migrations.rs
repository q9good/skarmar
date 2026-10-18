use rusqlite::Connection;
use skarma_api::store;

#[test]
fn unversioned_database_is_upgraded_without_replacing_data() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../../core/migrations/001_initial.sql"))
        .unwrap();
    db.execute("INSERT INTO goals(id,data) VALUES (?1,?2)", [
        "672f7f01-6699-442d-813b-1caa7a2a4715",
        r#"{"id":"672f7f01-6699-442d-813b-1caa7a2a4715","title":"legacy goal","level":"long","parent_id":null,"area":"","criteria":"","status":"active","version":1}"#,
    ]).unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let state = store::state(&db).unwrap();
    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.goals[0].title, "legacy goal");
    assert!(
        store::goal_history(&db, &state.goals[0].id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM schema_migrations", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
}

#[test]
fn incompatible_database_version_is_rejected_without_modification() {
    let db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    db.execute("INSERT INTO schema_migrations VALUES (99)", [])
        .unwrap();
    let error = store::initialize(&db).unwrap_err();
    assert!(error.1.contains("数据库版本"));
    assert_eq!(
        db.query_row("SELECT count(*) FROM schema_migrations", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        7
    );
}

#[test]
fn existing_accounts_survive_registration_migration() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../../core/migrations/001_initial.sql"))
        .unwrap();
    db.execute_batch(include_str!("../../core/migrations/002_goal_revisions.sql"))
        .unwrap();
    db.execute_batch(include_str!("../../core/migrations/003_auth.sql"))
        .unwrap();
    db.execute_batch(include_str!("../../core/migrations/004_pathways.sql"))
        .unwrap();
    db.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY); INSERT INTO schema_migrations(version) VALUES (1),(2),(3),(4);").unwrap();
    db.execute("INSERT INTO auth_users(id,username,display_name,password_hash,role,active,created_at) VALUES ('existing','owner','原管理员','old-hash','admin',1,'2026-01-01')", []).unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let account: (String, String, Option<String>) = db
        .query_row(
            "SELECT display_name,password_hash,email FROM auth_users WHERE id='existing'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(account, ("原管理员".into(), "old-hash".into(), None));
    assert_eq!(
        db.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        7
    );
}
