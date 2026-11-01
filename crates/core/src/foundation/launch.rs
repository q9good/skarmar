//! Explicit one-time trial cleanup, never an ordinary schema migration/startup.
//! The adapter MUST enclose the whole call and its completion marker in a transaction.
use crate::{
    sql::{Sql, params},
    store::{ApiError, Result, json},
};
use serde_json::{Value, json as value};

pub const EPOCH: &str = "formal-20261101";
pub const ACK: &str = "retain-goals-courses-teachers-accounts-20261101";
const MARKER: &str = "formal-launch-20261101";
pub const STALE_PAGE: &str = "试用版已结束，请刷新页面进入正式版。旧试用草稿不会自动上传。";

// Dependency order matters: sources are removed after outputs and revisions.
const CLEAR: &[&str] = &[
    "prd_annotations",
    "prd_feedback_revisions",
    "prd_output_revisions",
    "prd_knowledge",
    "prd_actions",
    "prd_feedback",
    "prd_review_revisions",
    "prd_reviews",
    "prd_lesson_refs",
    "prd_lesson_revisions",
    "prd_session_revisions",
    "prd_lessons",
    "prd_sessions",
    "prd_observation_revisions",
    "prd_observations",
    "prd_import_previews",
    "prd_import_rows",
    "prd_import_batches",
    "prd_method_goals",
];
const LEGACY_CLEAR: &[&str] = &[
    "experience_goals",
    "experience_pathways",
    "goal_pathways",
    "session_targets",
    "lesson_targets",
    "session_revisions",
    "difficulties",
    "experiences",
    "sessions",
    "lessons",
    "pathways",
    "operations",
    "deleted_records",
    "import_previews",
    "import_source_rows",
    "import_batches",
];

fn marker(db: &dyn Sql) -> Result<Option<Value>> {
    db.query("SELECT data FROM prd_upgrade WHERE id=?1", params![MARKER])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()
}
pub fn status(db: &dyn Sql) -> Result<Value> {
    let saved = marker(db)?;
    if let Some(m) = &saved
        && (m["schema"] != "skarmar.formal-launch.v1"
            || m["epoch"] != EPOCH
            || m["preserved_verified"] != true
            || m["cleared_verified"] != true)
    {
        return Err(ApiError::conflict(
            "正式启用标记异常，请先核对，禁止再次清理",
        ));
    }
    // No counts, identifiers, content, credentials, hashes or snapshots are public.
    Ok(value!({"epoch": EPOCH, "completed": saved.is_some()}))
}
pub fn require_epoch(db: &dyn Sql, epoch: Option<&str>) -> Result<()> {
    if marker(db)?.is_some() {
        status(db)?;
        if epoch != Some(EPOCH) {
            return Err(ApiError::conflict(STALE_PAGE));
        }
    }
    Ok(())
}
fn image(db: &dyn Sql) -> Result<Vec<Vec<Vec<String>>>> {
    let mut rows = [
        "SELECT json_array(id,username,display_name,password_hash,role,active,created_at,email) FROM auth_users ORDER BY id",
        "SELECT json_array(token_hash,user_id,created_at,expires_at) FROM auth_sessions ORDER BY token_hash",
        "SELECT json_array(id,username,display_name,email,password_hash,created_at) FROM auth_registration_requests ORDER BY id",
        "SELECT json_array(key,failures,window_start,locked_until) FROM auth_login_attempts ORDER BY key",
        "SELECT json_array(key,attempts,window_start) FROM auth_registration_limits ORDER BY key",
        "SELECT json_array(id,kind,version,name,data) FROM prd_entities WHERE kind IN ('goal','course','teacher','domain') ORDER BY id",
        "SELECT json_array(entity_id,version,operation_id,data) FROM prd_revisions WHERE entity_id IN (SELECT id FROM prd_entities WHERE kind IN ('goal','course','teacher','domain')) ORDER BY entity_id,version",
        "SELECT json_array(id,actor_id,request,response) FROM prd_operations WHERE id IN (SELECT operation_id FROM prd_revisions WHERE entity_id IN (SELECT id FROM prd_entities WHERE kind IN ('goal','course','teacher','domain'))) ORDER BY id",
        "SELECT data FROM prd_settings ORDER BY id",
        "SELECT json_array(object_kind,object_id,version,data) FROM prd_schedules WHERE object_kind='goal' ORDER BY object_id",
        "SELECT json_array(object_kind,object_id,version,data) FROM prd_schedule_revisions WHERE object_kind='goal' ORDER BY object_id,version",
    ].iter().map(|sql| db.query(sql, &[])).collect::<Result<Vec<_>>>()?;
    if !db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='goals'",
            &[],
        )?
        .is_empty()
    {
        for sql in [
            "SELECT json_array(id,parent_id,data) FROM goals ORDER BY id",
            "SELECT json_array(goal_id,version,data) FROM goal_revisions ORDER BY goal_id,version",
            "SELECT json_array(kind,value,active,version,renamed_to) FROM lesson_options ORDER BY kind,value",
        ] {
            rows.push(db.query(sql, &[])?);
        }
    }
    Ok(rows)
}
pub fn apply_once(db: &dyn Sql, acknowledgment: &str) -> Result<Value> {
    if acknowledgment != ACK {
        return Err(ApiError::invalid("缺少本次正式启用的精确清理确认"));
    }
    if !super::enabled(db)? {
        return Err(ApiError::conflict(
            "只能清理既有新模型，禁止初始化或转换数据库",
        ));
    }
    super::initialize_authenticated(db)?; // Refuses unknown schemas/tables, preserves existing accounts.
    if marker(db)?.is_some() {
        return status(db);
    }
    if db
        .query(
            "SELECT id FROM auth_users WHERE active=1 AND role='admin' LIMIT 1",
            &[],
        )?
        .is_empty()
    {
        return Err(ApiError::conflict(
            "正式启用必须保留既有有效管理员，不能创建新账号",
        ));
    }
    let before = image(db)?; // Private in-memory equality check, never logged or exported.
    for table in CLEAR {
        db.execute(&format!("DELETE FROM {table}"), &[])?;
    }
    db.execute("DELETE FROM prd_revisions WHERE entity_id IN (SELECT id FROM prd_entities WHERE kind='method')", &[])?;
    db.execute("DELETE FROM prd_entities WHERE kind='method'", &[])?;
    db.execute(
        "DELETE FROM prd_operations WHERE id NOT IN (SELECT operation_id FROM prd_revisions)",
        &[],
    )?;
    db.execute(
        "DELETE FROM prd_schedule_revisions WHERE object_kind<>'goal'",
        &[],
    )?;
    db.execute("DELETE FROM prd_schedules WHERE object_kind<>'goal'", &[])?;
    // Keep only technical legacy coexistence metadata; import/starter receipts are trial data.
    db.execute("DELETE FROM prd_upgrade WHERE id<>'legacy-v10'", &[])?;
    let legacy = !db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='goals'",
            &[],
        )?
        .is_empty();
    if legacy {
        for table in LEGACY_CLEAR {
            db.execute(&format!("DELETE FROM {table}"), &[])?;
        }
    }
    if image(db)? != before {
        return Err(ApiError::conflict("保留资料核对失败，整次清理必须回滚"));
    }
    for sql in [
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_entities WHERE kind='method'",
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_schedules WHERE object_kind<>'goal'",
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_schedule_revisions WHERE object_kind<>'goal'",
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_operations WHERE id NOT IN (SELECT operation_id FROM prd_revisions)",
        "SELECT CAST(COUNT(*) AS TEXT) FROM prd_upgrade WHERE id<>'legacy-v10'",
    ] {
        if db.query(sql, &[])?[0][0] != "0" {
            return Err(ApiError::conflict("仍有试用内容残留，整次清理必须回滚"));
        }
    }
    for table in CLEAR.iter().chain(if legacy { LEGACY_CLEAR } else { &[] }) {
        if db.query(&format!("SELECT CAST(COUNT(*) AS TEXT) FROM {table}"), &[])?[0][0] != "0" {
            return Err(ApiError::conflict("试用资料清理核对失败，整次操作必须回滚"));
        }
    }
    if !db.query("PRAGMA foreign_key_check", &[])?.is_empty() {
        return Err(ApiError::conflict("清理后的引用核对失败，整次操作必须回滚"));
    }
    db.execute(
        "INSERT INTO prd_upgrade(id,data) VALUES(?1,?2)",
        params![
            MARKER,
            json(&value!({
                "schema":"skarmar.formal-launch.v1", "epoch":EPOCH,
                "completed_at":chrono::Utc::now().to_rfc3339(),
                "preserved_verified":true, "cleared_verified":true
            }))?
        ],
    )?;
    status(db)
}
