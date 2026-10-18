use axum::http::StatusCode;
use rusqlite::Connection;
use skarma_api::{model::*, store};
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
    }
}

#[test]
fn records_a_class_without_a_training_plan_and_supports_safe_edits() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let id = Uuid::new_v4().to_string();
    let request = SaveLesson {
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
