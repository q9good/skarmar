use rusqlite::Connection;
use skarma_api::store;
use skarma_core::{
    Command,
    auth::{AccountRole, AuthCommand, NewUser},
    import::{
        ImportCommit, ImportCommitInput, ImportPreview, ImportPreviewInput, ImportRow, ImportSheet,
    },
};
use std::sync::{Arc, Mutex};

fn signed(db: &mut Connection, token: &str, command: Command) -> serde_json::Value {
    store::execute(
        db,
        Command::Authenticated {
            token: token.into(),
            command: Box::new(command),
        },
    )
    .unwrap()
}

fn admin(db: &mut Connection) -> String {
    store::bootstrap_admin(
        db,
        NewUser {
            username: "owner".into(),
            display_name: "管理员".into(),
            password: "correct-horse-battery-staple".into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    let value = store::execute(
        db,
        Command::Auth(AuthCommand::Login {
            username: "owner".into(),
            password: "correct-horse-battery-staple".into(),
        }),
    )
    .unwrap();
    value["token"].as_str().unwrap().to_owned()
}

fn account(db: &mut Connection, owner: &str, username: &str, role: AccountRole) -> String {
    store::execute(
        db,
        Command::Auth(AuthCommand::CreateUser {
            token: owner.into(),
            input: NewUser {
                username: username.into(),
                display_name: "合成测试成员".into(),
                password: "synthetic-import-password".into(),
                role,
            },
        }),
    )
    .unwrap();
    let value = store::execute(
        db,
        Command::Auth(AuthCommand::Login {
            username: username.into(),
            password: "synthetic-import-password".into(),
        }),
    )
    .unwrap();
    value["token"].as_str().unwrap().to_owned()
}

fn denied(db: &mut Connection, token: &str, command: Command) -> axum::http::StatusCode {
    store::execute(
        db,
        Command::Authenticated {
            token: token.into(),
            command: Box::new(command),
        },
    )
    .unwrap_err()
    .0
}

fn count(db: &Connection, table: &str) -> i64 {
    // Table names are test constants, never uploaded content.
    db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn source(observation: &str) -> ImportPreviewInput {
    ImportPreviewInput {
        filename: "个训记录.xlsx".into(),
        sha256: "test-file".into(),
        sheets: vec![
            ImportSheet {
                name: "阶段目标".into(),
                headers: vec![
                    "阶段目标名称".into(),
                    "验收标准".into(),
                    "训练领域".into(),
                    "状态".into(),
                ],
                rows: vec![ImportRow {
                    row: 2,
                    cells: vec![
                        "主动表达需求".into(),
                        "主动说出".into(),
                        "语言".into(),
                        "进行中".into(),
                    ],
                }],
            },
            ImportSheet {
                name: "每日训练记录".into(),
                headers: vec![
                    "日期".into(),
                    "时间段".into(),
                    "阶段性训练重点".into(),
                    "训练记录".into(),
                    "存在问题".into(),
                    "下一步计划".into(),
                ],
                rows: vec![ImportRow {
                    row: 2,
                    cells: vec![
                        "2026-10-17".into(),
                        "上午".into(),
                        "表达练习".into(),
                        observation.into(),
                        "需要提示".into(),
                        "逐渐减少提示".into(),
                    ],
                }],
            },
        ],
    }
}

#[test]
fn preview_is_not_business_write_and_commit_is_idempotent() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let token = admin(&mut db);
    let preview: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &token,
        Command::ImportPreview(source("能够回应")),
    ))
    .unwrap();
    assert!(preview.can_commit);
    assert_eq!(preview.totals.new, 2);
    assert_eq!(store::state(&db).unwrap().goals.len(), 0);
    assert_eq!(store::state(&db).unwrap().sessions.len(), 0);
    let command = Command::ImportCommit(ImportCommitInput {
        preview_id: preview.preview_id,
    });
    let committed: ImportCommit =
        serde_json::from_value(signed(&mut db, &token, command.clone())).unwrap();
    assert_eq!(committed.totals.new, 2);
    let repeated: ImportCommit = serde_json::from_value(signed(&mut db, &token, command)).unwrap();
    assert_eq!(committed.batch_id, repeated.batch_id);
    let state = store::state(&db).unwrap();
    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.sessions.len(), 1);
    assert!(state.sessions[0].target_snapshots.is_empty());
    assert!(state.sessions[0].input.plan.contains("逐渐减少提示"));
    let preview: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &token,
        Command::ImportPreview(source("能够回应")),
    ))
    .unwrap();
    assert_eq!(preview.totals.skipped, 2);
    assert!(!preview.can_commit);
    let changed: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &token,
        Command::ImportPreview(source("不同的观察")),
    ))
    .unwrap();
    assert_eq!(changed.totals.invalid, 1);
    assert!(!changed.can_commit);
}

#[test]
fn rejects_unwrapped_import_command() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let error = store::execute(&mut db, Command::ImportPreview(source("训练观察"))).unwrap_err();
    assert_eq!(error.0, axum::http::StatusCode::UNAUTHORIZED);
    for command in [
        Command::ImportAuthorize,
        Command::ImportCommit(ImportCommitInput {
            preview_id: uuid::Uuid::new_v4().to_string(),
        }),
    ] {
        assert_eq!(
            store::execute(&mut db, command).unwrap_err().0,
            axum::http::StatusCode::UNAUTHORIZED
        );
    }
}

#[test]
fn member_cannot_preview_or_commit_an_admins_upload() {
    use axum::http::StatusCode;
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let owner = admin(&mut db);
    let member = account(&mut db, &owner, "family-member", AccountRole::Member);
    for command in [
        Command::ImportAuthorize,
        Command::ImportPreview(source("权限测试")),
    ] {
        assert_eq!(denied(&mut db, &member, command), StatusCode::FORBIDDEN);
    }
    assert_eq!(count(&db, "import_previews"), 0);
    let preview: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &owner,
        Command::ImportPreview(source("权限测试")),
    ))
    .unwrap();
    assert_eq!(
        denied(
            &mut db,
            &member,
            Command::ImportCommit(ImportCommitInput {
                preview_id: preview.preview_id,
            }),
        ),
        StatusCode::FORBIDDEN
    );
    assert_eq!(count(&db, "import_batches"), 0);
    assert_eq!(count(&db, "goals"), 0);
    assert_eq!(count(&db, "sessions"), 0);
}

#[test]
fn preview_expiry_and_owner_are_checked_before_any_business_write() {
    use axum::http::StatusCode;
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let owner = admin(&mut db);
    let other_admin = account(&mut db, &owner, "other-admin", AccountRole::Admin);
    let preview: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &owner,
        Command::ImportPreview(source("有效预览")),
    ))
    .unwrap();
    let command = Command::ImportCommit(ImportCommitInput {
        preview_id: preview.preview_id.clone(),
    });
    assert_eq!(
        denied(&mut db, &other_admin, command.clone()),
        StatusCode::GONE
    );
    assert_eq!(count(&db, "import_previews"), 1);
    db.execute(
        "UPDATE import_previews SET expires_at=0 WHERE id=?1",
        [&preview.preview_id],
    )
    .unwrap();
    assert_eq!(denied(&mut db, &owner, command), StatusCode::GONE);
    for table in ["goals", "sessions", "import_batches", "import_source_rows"] {
        assert_eq!(count(&db, table), 0, "expired preview wrote {table}");
    }
}

#[test]
fn late_sql_failure_rolls_back_batch_goals_revisions_and_source_rows_and_can_retry() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let owner = admin(&mut db);
    let preview: ImportPreview = serde_json::from_value(signed(
        &mut db,
        &owner,
        Command::ImportPreview(source("回滚后的重试")),
    ))
    .unwrap();
    let command = Command::ImportCommit(ImportCommitInput {
        preview_id: preview.preview_id,
    });
    // The first row already wrote a goal, revision and source row when the
    // second row fails. A failed batch must leave its preview available.
    db.execute_batch(
        "CREATE TRIGGER fail_import_session BEFORE INSERT ON sessions
         BEGIN SELECT RAISE(ABORT,'synthetic import failure'); END;",
    )
    .unwrap();
    assert_eq!(
        denied(&mut db, &owner, command.clone()),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    for table in [
        "goals",
        "goal_revisions",
        "sessions",
        "import_batches",
        "import_source_rows",
    ] {
        assert_eq!(count(&db, table), 0, "failed transaction left {table}");
    }
    assert_eq!(count(&db, "import_previews"), 1);
    db.execute_batch("DROP TRIGGER fail_import_session;")
        .unwrap();
    let committed = signed(&mut db, &owner, command.clone());
    assert_eq!(signed(&mut db, &owner, command), committed);
    assert_eq!(count(&db, "goals"), 1);
    assert_eq!(count(&db, "goal_revisions"), 1);
    assert_eq!(count(&db, "sessions"), 1);
    assert_eq!(count(&db, "import_batches"), 1);
    assert_eq!(count(&db, "import_source_rows"), 2);
    assert_eq!(count(&db, "import_previews"), 0);
}

#[test]
fn unknown_sheet_blocks_the_whole_batch_instead_of_silently_archiving_it() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let owner = admin(&mut db);
    let mut input = source("有效的合成观察");
    input.sheets.push(ImportSheet {
        name: "尚未支持的新表".into(),
        // A related-record table may mention a goal without defining one.
        // Familiar column names cannot authorize an unverified table mapping.
        headers: vec!["目标名称".into()],
        rows: vec![ImportRow {
            row: 2,
            cells: vec!["不可丢失的合成原文".into()],
        }],
    });
    let preview: ImportPreview =
        serde_json::from_value(signed(&mut db, &owner, Command::ImportPreview(input))).unwrap();
    assert_eq!(preview.totals.new, 2);
    assert_eq!(preview.totals.invalid, 1);
    assert!(!preview.can_commit);
    assert_eq!(
        denied(
            &mut db,
            &owner,
            Command::ImportCommit(ImportCommitInput {
                preview_id: preview.preview_id,
            }),
        ),
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    for table in ["goals", "sessions", "import_batches", "import_source_rows"] {
        assert_eq!(count(&db, table), 0, "invalid batch partly wrote {table}");
    }
}

#[tokio::test]
async fn multipart_preview_accepts_file_larger_than_default_json_limit() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    let mut connection = Connection::open_in_memory().unwrap();
    store::initialize(&connection).unwrap();
    let token = admin(&mut connection);
    let db = skarma_api::Database(Arc::new(Mutex::new(connection)));
    let app = skarma_api::router_with_auth(
        db,
        "https://app.example.test".parse().unwrap(),
        "/tmp/skarma-no-assets",
        skarma_http::auth::AuthConfig {
            demo: false,
            secure_cookie: true,
            origins: vec!["https://app.example.test".into()],
            space: "family".into(),
        },
    );
    let mut csv = String::from("日期,训练记录\n");
    for index in 0..200 {
        csv.push_str(&format!("2026-10-17,{}-{index}\n", "观察".repeat(300)));
    }
    assert!(csv.len() > 128 * 1024);
    let body = format!(
        "--test-boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"每日训练记录.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--test-boundary--\r\n"
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/import/preview")
        .header(
            "content-type",
            "multipart/form-data; boundary=test-boundary",
        )
        .header("x-skarma-request", "1")
        .header("origin", "https://app.example.test")
        .header("cookie", format!("__Host-skarma_session={token}"))
        .body(Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let response_body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&response_body)
    );
    let response: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
    assert_eq!(response["totals"]["new"], 200);
}

#[test]
fn oversized_serialized_preview_is_rejected_before_storage() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let token = admin(&mut db);
    let mut input = source(&"x".repeat(20_000));
    let row = input.sheets[1].rows[0].clone();
    input.sheets[1].rows = (2..62)
        .map(|index| ImportRow {
            row: index,
            cells: row.cells.clone(),
        })
        .collect();
    let result = store::execute(
        &mut db,
        Command::Authenticated {
            token,
            command: Box::new(Command::ImportPreview(input)),
        },
    );
    assert_eq!(
        result.unwrap_err().0,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM import_previews", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM import_batches", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(store::state(&db).unwrap().sessions.is_empty());
}

#[test]
fn repeated_large_headers_are_bounded_before_source_archive_expansion() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let token = admin(&mut db);
    let mut input = source("合成观察");
    input.sheets.truncate(1);
    let sheet = &mut input.sheets[0];
    for index in 0..90 {
        sheet.headers.push(format!("{index}{}", "x".repeat(5_000)));
    }
    sheet.rows = (2..12)
        .map(|index| ImportRow {
            row: index,
            cells: vec![
                format!("合成目标{index}"),
                "标准".into(),
                "领域".into(),
                "进行中".into(),
            ],
        })
        .collect();
    assert!(serde_json::to_vec(&input).unwrap().len() < 1_000_000);
    let result = store::execute(
        &mut db,
        Command::Authenticated {
            token,
            command: Box::new(Command::ImportPreview(input)),
        },
    );
    assert_eq!(
        result.unwrap_err().0,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM import_previews", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(store::state(&db).unwrap().goals.is_empty());
}
