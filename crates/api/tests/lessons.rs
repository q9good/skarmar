use axum::http::StatusCode;
use rusqlite::Connection;
use skarma_api::{model::*, store};
use skarma_core::Command;
use uuid::Uuid;

fn lesson() -> LessonInput {
    LessonInput {
        business_date: "2026-10-18".into(),
        course: "  音乐课  ".into(),
        teacher: "  王老师 ".into(),
        content: "唱歌".into(),
        goals: "跟唱".into(),
        problems: String::new(),
        home_cooperation: String::new(),
        links: None,
    }
}

#[test]
fn shared_lesson_options_are_idempotent_versioned_and_do_not_rewrite_history() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let id = Uuid::new_v4().to_string();
    store::save_lesson(
        &mut db,
        &id,
        SaveLesson {
            difficulty_version: None,
            expected_version: None,
            lesson: lesson(),
        },
    )
    .unwrap();
    let original = store::state(&db).unwrap().lessons;
    let option: LessonOption = serde_json::from_value(
        store::execute(
            &mut db,
            Command::AddLessonOption(NewLessonOption {
                kind: "teacher".into(),
                value: " 王老师 ".into(),
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(option.version, 1);
    assert_eq!(store::state(&db).unwrap().lesson_options.len(), 2);
    let change = UpdateLessonOption {
        new_value: None,
        kind: option.kind.clone(),
        value: option.value.clone(),
        active: false,
        expected_version: option.version,
    };
    let stopped = store::execute(&mut db, Command::UpdateLessonOption(change.clone())).unwrap();
    assert_eq!(stopped["active"], false);
    assert_eq!(stopped["version"], 2);
    assert_eq!(
        store::execute(&mut db, Command::UpdateLessonOption(change))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    let retry = store::execute(
        &mut db,
        Command::AddLessonOption(NewLessonOption {
            kind: "teacher".into(),
            value: option.value,
        }),
    )
    .unwrap();
    assert_eq!(
        retry["active"], false,
        "saving a historical name must not reactivate it"
    );
    assert_eq!(
        serde_json::to_value(store::state(&db).unwrap().lessons).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    let invalid = store::execute(
        &mut db,
        Command::AddLessonOption(NewLessonOption {
            kind: "teacher".into(),
            value: " ".into(),
        }),
    )
    .unwrap_err();
    assert_eq!(invalid.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[test]
fn catalog_migration_preserves_existing_lessons_and_repeated_startup() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let id = Uuid::new_v4().to_string();
    let saved = store::save_lesson(
        &mut db,
        &id,
        SaveLesson {
            difficulty_version: None,
            expected_version: None,
            lesson: lesson(),
        },
    )
    .unwrap();
    db.execute_batch("DROP TABLE lesson_targets; DROP INDEX one_difficulty_per_lesson; DROP TABLE lesson_options; DELETE FROM schema_migrations WHERE version IN (8,10);")
        .unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let state = store::state(&db).unwrap();
    assert_eq!(
        serde_json::to_value(&state.lessons[0]).unwrap(),
        serde_json::to_value(saved).unwrap()
    );
    assert_eq!(state.lesson_options.len(), 2);
    assert!(
        state
            .lesson_options
            .iter()
            .all(|item| item.active && item.version == 1)
    );
}

#[test]
fn records_a_class_without_a_training_plan_and_supports_safe_edits() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let id = Uuid::new_v4().to_string();
    let request = SaveLesson {
        difficulty_version: None,
        expected_version: None,
        lesson: lesson(),
    };
    let first = store::save_lesson(&mut db, &id, request.clone()).unwrap();
    assert_eq!(first.version, 1);
    assert_eq!(first.input.course, "音乐课");
    assert_eq!(first.input.teacher, "王老师");
    assert_eq!(
        store::save_lesson(&mut db, &id, request).unwrap().version,
        1
    );
    assert!(store::state(&db).unwrap().sessions.is_empty());

    let mut revised = first.input.clone();
    revised.home_cooperation = "每天练习十分钟".into();
    let update = SaveLesson {
        difficulty_version: None,
        expected_version: Some(1),
        lesson: revised.clone(),
    };
    let second = store::save_lesson(&mut db, &id, update.clone()).unwrap();
    assert_eq!(second.version, 2);
    assert_eq!(store::save_lesson(&mut db, &id, update).unwrap().version, 2);
    assert_eq!(
        store::state(&db).unwrap().lessons[0].input.home_cooperation,
        "每天练习十分钟"
    );
    revised.content = "另一人的更新".into();
    assert_eq!(
        store::save_lesson(
            &mut db,
            &id,
            SaveLesson {
                difficulty_version: None,
                expected_version: Some(1),
                lesson: revised
            }
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
}

#[test]
fn course_teacher_and_date_are_validated_before_writing() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let mut no_course = lesson();
    no_course.course = "  ".into();
    assert_eq!(
        store::save_lesson(
            &mut db,
            &Uuid::new_v4().to_string(),
            SaveLesson {
                difficulty_version: None,
                expected_version: None,
                lesson: no_course
            }
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut no_teacher = lesson();
    no_teacher.teacher = "  ".into();
    assert_eq!(
        store::save_lesson(
            &mut db,
            &Uuid::new_v4().to_string(),
            SaveLesson {
                difficulty_version: None,
                expected_version: None,
                lesson: no_teacher
            }
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut invalid_date = lesson();
    invalid_date.business_date = "2026-02-30".into();
    assert_eq!(
        store::save_lesson(
            &mut db,
            &Uuid::new_v4().to_string(),
            SaveLesson {
                difficulty_version: None,
                expected_version: None,
                lesson: invalid_date
            }
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(store::state(&db).unwrap().lessons.is_empty());
}

#[test]
fn version_nine_upgrade_keeps_raw_lesson_text_and_inactive_catalog_options() {
    let db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    db.execute_batch("DROP TABLE lesson_targets; DROP INDEX one_difficulty_per_lesson; ALTER TABLE lesson_options DROP COLUMN renamed_to; DELETE FROM schema_migrations WHERE version=10;").unwrap();
    let id = Uuid::new_v4().to_string();
    let raw = serde_json::json!({"id":id,"version":3,"business_date":"2026-10-21","course":"历史课程","teacher":"历史老师","content":"逐字保留原文","goals":"未关联目标原文","problems":"问题备注","home_cooperation":"配合内容"}).to_string();
    db.execute(
        "INSERT INTO lessons(id,business_date,data) VALUES (?1,'2026-10-21',?2)",
        [&id, &raw],
    )
    .unwrap();
    db.execute(
        "INSERT INTO lesson_options(kind,value,active,version) VALUES ('teacher','历史老师',0,4)",
        [],
    )
    .unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let state = store::state(&db).unwrap();
    assert!(state.lessons[0].input.links.is_none());
    assert!(state.lessons[0].target_snapshots.is_empty());
    assert_eq!(state.lesson_options[0].version, 4);
    assert!(!state.lesson_options[0].active);
    assert!(state.lesson_options[0].renamed_to.is_none());
    assert_eq!(
        db.query_row("SELECT data FROM lessons WHERE id=?1", [&id], |r| r
            .get::<_, String>(0))
            .unwrap(),
        raw
    );
}
