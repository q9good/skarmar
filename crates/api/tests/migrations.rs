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
        10
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
        11
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
        10
    );
}

#[test]
fn import_migration_backfills_a_version_seven_database_without_replacing_data() {
    let db = Connection::open_in_memory().unwrap();
    // Reproduce the deployed migration set, where version six was reserved.
    for sql in [
        include_str!("../../core/migrations/001_initial.sql"),
        include_str!("../../core/migrations/002_goal_revisions.sql"),
        include_str!("../../core/migrations/003_auth.sql"),
        include_str!("../../core/migrations/004_pathways.sql"),
        include_str!("../../core/migrations/005_registration.sql"),
        include_str!("../../core/migrations/007_lessons.sql"),
    ] {
        db.execute_batch(sql).unwrap();
    }
    db.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY); INSERT INTO schema_migrations VALUES (1),(2),(3),(4),(5),(7);").unwrap();
    db.execute("INSERT INTO auth_users(id,username,display_name,password_hash,role,active,created_at) VALUES ('legacy-admin','owner','原管理员','unchanged-hash','admin',1,'2026-01-01')", []).unwrap();
    let lesson = r#"{"id":"legacy-lesson","version":2,"business_date":"2026-10-19","course_name":"合成课程","teacher":"合成老师","content":"已有课程原文"}"#;
    db.execute(
        "INSERT INTO lessons(id,business_date,data) VALUES (?1,?2,?3)",
        ["legacy-lesson", "2026-10-19", lesson],
    )
    .unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let versions: Vec<i64> = db
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(
        db.query_row(
            "SELECT password_hash FROM auth_users WHERE id='legacy-admin'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "unchanged-hash"
    );
    assert_eq!(
        db.query_row(
            "SELECT data FROM lessons WHERE id='legacy-lesson'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        lesson
    );
    for table in ["import_previews", "import_batches", "import_source_rows"] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
