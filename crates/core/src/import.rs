//! An uploaded spreadsheet is parsed into plain cells before it reaches either
//! SQL backend. The original column names and values are retained for audit.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    auth::{AccountRole, User},
    model::{Goal, GoalProgress, GoalStatus, Level, Session, SessionInput},
    sql::{Sql, params},
    store::{self, ApiError, Result},
};
use axum::http::StatusCode;

const DOCUMENT_ID: &str = "DVENwZGhrRUpka2xN";
const PREVIEW_SECONDS: i64 = 60 * 60;
// Keep a stored preview comfortably below a SQLite-backed Durable Object's
// per-row bound, including UTF-8 text and JSON escaping. Both backends agree.
const MAX_PREVIEW_BYTES: usize = 1_000_000;
const MAX_SOURCE_ARCHIVE_BYTES: usize = 4_000_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportRow {
    /// One-based row number in the original worksheet.
    pub row: usize,
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportSheet {
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<ImportRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportPreviewInput {
    pub filename: String,
    pub sha256: String,
    pub sheets: Vec<ImportSheet>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportCommitInput {
    pub preview_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ImportTotals {
    pub rows: usize,
    pub new: usize,
    pub updated: usize,
    pub skipped: usize,
    pub invalid: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportWarning {
    pub sheet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<usize>,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportColumn {
    pub source: String,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportSheetPreview {
    pub name: String,
    pub target: Option<String>,
    pub rows: usize,
    pub new: usize,
    pub updated: usize,
    pub skipped: usize,
    pub invalid: usize,
    pub columns: Vec<ImportColumn>,
    pub sample: Vec<ImportSample>,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportSample {
    pub row: usize,
    pub label: String,
    pub action: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportPreview {
    pub preview_id: String,
    pub filename: String,
    pub expires_at: String,
    pub totals: ImportTotals,
    pub sheets: Vec<ImportSheetPreview>,
    pub warnings: Vec<ImportWarning>,
    pub can_commit: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportCommit {
    pub batch_id: String,
    pub totals: ImportTotals,
    pub sheets: Vec<ImportSheetPreview>,
    pub warnings: Vec<ImportWarning>,
}

pub fn ensure_admin(actor: &User) -> Result<()> {
    if actor.role != AccountRole::Admin {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "只有管理员可以导入数据".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetKind {
    Goal,
    Session,
    Archive,
}
impl TargetKind {
    fn label(self) -> &'static str {
        match self {
            Self::Goal => "目标",
            Self::Session => "训练记录",
            Self::Archive => "原始资料存档",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Session => "session",
            Self::Archive => "archive",
        }
    }
}

#[derive(Clone)]
struct Mapping {
    kind: TargetKind,
    record_id: Option<usize>,
    title: Option<usize>,
    level: Option<usize>,
    area: Option<usize>,
    criteria: Option<usize>,
    progress: Option<usize>,
    archived: Option<usize>,
    start: Option<usize>,
    due: Option<usize>,
    date: Option<usize>,
    time_slot: Option<usize>,
    focus: Option<usize>,
    observation: Option<usize>,
    problem: Option<usize>,
    next_plan: Option<usize>,
    recorder: Option<usize>,
    training_format: Option<usize>,
    source_links: Vec<(String, usize)>,
    columns: Vec<ImportColumn>,
}

fn find_column(headers: &[String], aliases: &[&str]) -> Result<Option<usize>> {
    let matches = headers
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            aliases
                .iter()
                .any(|alias| name.trim().eq_ignore_ascii_case(alias))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(ApiError::invalid(
            "导入列名重复或含同义列，请先在表格中整理",
        ));
    }
    Ok(matches.first().copied())
}

fn mapping(sheet: &ImportSheet) -> Result<Mapping> {
    let headers = &sheet.headers;
    if headers.is_empty() || headers.iter().any(|name| name.trim().is_empty()) {
        return Err(ApiError::invalid("工作表的表头不能为空"));
    }
    let mut unique = HashSet::new();
    if headers
        .iter()
        .any(|name| !unique.insert(name.trim().to_lowercase()))
    {
        return Err(ApiError::invalid("工作表包含重名列，无法安全映射"));
    }
    let goal_name = matches!(sheet.name.trim(), "阶段目标" | "目标总表" | "目标");
    let session_name = matches!(sheet.name.trim(), "每日训练记录" | "训练记录");
    // A related table can also contain a target-name or training-text column.
    // Only reviewed table names select semantics; column similarity is unsafe.
    let kind = match (goal_name, session_name) {
        (true, false) => TargetKind::Goal,
        (false, true) => TargetKind::Session,
        (false, false) if sheet.name.trim() == "使用说明" => TargetKind::Archive,
        _ => {
            return Err(ApiError::invalid(
                "此工作表尚未建立可靠映射，请先核对表名与字段；不会自动写入原始存档或业务记录",
            ));
        }
    };
    let mut mapped = HashMap::new();
    macro_rules! col {
        ($aliases:expr, $label:expr) => {{
            let found = find_column(headers, $aliases)?;
            if let Some(index) = found {
                mapped.insert(index, $label.to_owned());
            }
            found
        }};
    }
    let record_id = if kind == TargetKind::Goal {
        col!(
            &[
                "记录ID",
                "记录 ID",
                "record_id",
                "Record ID",
                "ID",
                "目标编号"
            ],
            "来源记录 ID"
        )
    } else {
        col!(
            &["记录ID", "记录 ID", "record_id", "Record ID", "ID"],
            "来源记录 ID"
        )
    };
    let mut output = Mapping {
        kind,
        record_id,
        title: None,
        level: None,
        area: None,
        criteria: None,
        progress: None,
        archived: None,
        start: None,
        due: None,
        date: None,
        time_slot: None,
        focus: None,
        observation: None,
        problem: None,
        next_plan: None,
        recorder: None,
        training_format: None,
        source_links: vec![],
        columns: vec![],
    };
    match kind {
        TargetKind::Goal => {
            output.title = col!(&["阶段目标名称", "目标名称", "名称"], "目标名称");
            output.level = col!(&["目标层级", "层级", "目标类型"], "目标层级");
            output.area = col!(&["训练领域", "领域", "目标领域"], "训练领域");
            output.criteria = col!(&["验收标准", "达成标准", "评估标准"], "验收标准");
            output.progress = col!(&["状态", "训练状态", "培养状态", "目标状态"], "目标进度");
            output.archived = col!(&["归档状态", "是否归档"], "归档状态");
            output.start = col!(&["开始日期", "起始日期"], "开始日期");
            output.due = col!(&["结束日期", "截止日期", "预期完成日期"], "截止日期");
            for (index, header) in headers.iter().enumerate() {
                if matches!(header.trim(), "关联" | "关联 2" | "关联2") {
                    mapped.insert(index, "来源关联原文（保留，不推断主辅角色）".into());
                    output.source_links.push((header.clone(), index));
                }
            }
        }
        TargetKind::Session => {
            output.date = col!(&["日期", "训练日期", "记录日期"], "训练日期");
            output.time_slot = col!(&["时间段", "训练时段", "时间"], "时间段");
            output.focus = col!(&["阶段性训练重点", "训练重点", "训练计划"], "训练计划");
            output.observation = col!(&["训练记录", "训练观察", "训练内容"], "训练观察");
            output.problem = col!(&["存在问题", "困难", "问题"], "困难原文");
            output.next_plan = col!(&["下一步计划", "后续计划"], "训练计划（下一步原文）");
            output.recorder = col!(&["记录人"], "观察备注（原表记录人）");
            output.training_format = col!(&["训练形式"], "观察备注（原表训练形式）");
        }
        TargetKind::Archive => {}
    }
    output.columns = headers
        .iter()
        .enumerate()
        .map(|(index, source)| ImportColumn {
            source: source.clone(),
            target: mapped.get(&index).cloned(),
        })
        .collect();
    Ok(output)
}

fn value(row: &ImportRow, index: Option<usize>) -> &str {
    index
        .and_then(|index| row.cells.get(index))
        .map(String::as_str)
        .unwrap_or("")
        .trim()
}

fn date(raw: &str) -> Result<Option<String>> {
    if raw.is_empty() {
        return Ok(None);
    }
    let short = raw.split([' ', 'T']).next().unwrap_or(raw);
    let normal = short
        .replace(['年', '月', '/', '.'], "-")
        .trim_end_matches('日')
        .to_owned();
    for fmt in ["%Y-%m-%d", "%Y-%-m-%-d"] {
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&normal, fmt) {
            return Ok(Some(parsed.format("%Y-%m-%d").to_string()));
        }
    }
    if let Ok(serial) = raw.parse::<f64>()
        && serial.is_finite()
        && (20_000.0..100_000.0).contains(&serial)
        && let Some(base) = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)
        && let Some(parsed) = base.checked_add_signed(chrono::Duration::days(serial.floor() as i64))
    {
        return Ok(Some(parsed.format("%Y-%m-%d").to_string()));
    }
    Err(ApiError::invalid(
        "日期格式无法识别，请在源表中使用 YYYY-MM-DD",
    ))
}

fn goal(row: &ImportRow, map: &Mapping) -> Result<Goal> {
    let title = value(row, map.title);
    if title.is_empty() || title.len() > 300 {
        return Err(ApiError::invalid("目标名称为空或超过 300 字节"));
    }
    let level = match value(row, map.level) {
        "" | "阶段目标" | "短期" | "短期目标" => Level::Short,
        "长期" | "长期目标" => Level::Long,
        "中期" | "中期目标" => Level::Medium,
        _ => {
            return Err(ApiError::invalid(
                "目标层级无法识别，请在源表中明确长期／中期／短期",
            ));
        }
    };
    let progress = match value(row, map.progress) {
        "" => None,
        "持续培养中" if level == Level::Long => Some(GoalProgress::Cultivating),
        "已内化" if level == Level::Long => Some(GoalProgress::Internalized),
        "未开始" if level != Level::Long => Some(GoalProgress::NotStarted),
        "进行中" if level != Level::Long => Some(GoalProgress::InProgress),
        "已达成" if level != Level::Long => Some(GoalProgress::Achieved),
        "已完成" if level != Level::Long => Some(GoalProgress::Completed),
        "已逾期" if level != Level::Long => Some(GoalProgress::Overdue),
        _ => return Err(ApiError::invalid("目标状态与层级不匹配，或状态值无法识别")),
    };
    let status = match value(row, map.archived) {
        "" | "活动" | "未归档" | "否" => GoalStatus::Active,
        "已归档" | "归档" | "是" if level != Level::Long => GoalStatus::Archived,
        _ => return Err(ApiError::invalid("归档状态无法识别，或长期目标不应归档")),
    };
    let criteria = value(row, map.criteria);
    let area = value(row, map.area);
    if criteria.len() > 5_000 || area.len() > 300 {
        return Err(ApiError::invalid("目标领域或验收标准过长"));
    }
    Ok(Goal {
        id: Uuid::new_v4().to_string(),
        title: title.into(),
        level,
        parent_id: None,
        area: area.into(),
        criteria: criteria.into(),
        status,
        version: 1,
        progress_status: progress,
        start_date: date(value(row, map.start))?,
        due_date: date(value(row, map.due))?,
        last_review_date: None,
        next_review_date: None,
        completion_date: None,
        review_notes: map
            .source_links
            .iter()
            .filter_map(|(label, index)| {
                let text = value(row, Some(*index));
                (!text.is_empty()).then(|| format!("原表{label}（原文，未推断主辅角色）：{text}"))
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    })
}

fn session(row: &ImportRow, map: &Mapping) -> Result<Session> {
    let business_date =
        date(value(row, map.date))?.ok_or_else(|| ApiError::invalid("训练日期为空"))?;
    if value(row, map.observation).is_empty() {
        return Err(ApiError::invalid(
            "训练记录为空，不能作为已完成训练导入；请核对或从导入副本中排除",
        ));
    }
    let focus = value(row, map.focus);
    let next = value(row, map.next_plan);
    let plan = match (focus.is_empty(), next.is_empty()) {
        (false, false) => format!("{focus}\n\n下一步计划：{next}"),
        (false, true) => focus.into(),
        (true, false) => format!("下一步计划：{next}"),
        (true, true) => String::new(),
    };
    let mut observation = value(row, map.observation).to_owned();
    for (label, index) in [("记录人", map.recorder), ("训练形式", map.training_format)] {
        let text = value(row, index);
        if !text.is_empty() {
            if !observation.is_empty() {
                observation.push_str("\n\n");
            }
            observation.push_str(&format!("原表{label}：{text}"));
        }
    }
    let difficulty = value(row, map.problem);
    if plan.len() > 30_000 || observation.len() > 30_000 || difficulty.len() > 20_000 {
        return Err(ApiError::invalid("训练记录正文过长，请拆分后导入"));
    }
    Ok(Session {
        id: Uuid::new_v4().to_string(),
        version: 1,
        // Historical rows have no verified per-target roles or outcomes; they
        // are read-only and intentionally have no fabricated target snapshots.
        status: "completed".into(),
        input: SessionInput {
            business_date,
            time_slot: value(row, map.time_slot).into(),
            activities: vec![],
            organization: String::new(),
            plan,
            observation,
            targets: vec![],
            has_difficulty: !difficulty.is_empty(),
            difficulty: difficulty.into(),
            has_experience: false,
            experience: String::new(),
            experience_kind: String::new(),
            difficulty_goal_ids: vec![],
            difficulty_pathway_ids: vec![],
            experience_goal_ids: vec![],
            experience_pathway_ids: vec![],
        },
        target_snapshots: vec![],
    })
}

fn sha(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[derive(Clone)]
struct PendingRow {
    sheet: String,
    row_key: String,
    raw_sha256: String,
    raw_json: String,
    kind: TargetKind,
    goal: Option<Goal>,
    session: Option<Session>,
}

fn warn(sheet: &str, row: Option<usize>, message: impl Into<String>) -> ImportWarning {
    ImportWarning {
        sheet: sheet.into(),
        row,
        message: message.into(),
    }
}

fn analyze(db: &dyn Sql, input: &ImportPreviewInput) -> Result<(ImportPreview, Vec<PendingRow>)> {
    let existing = db.query("SELECT source_sheet,source_row_key,raw_sha256 FROM import_source_rows WHERE source_document=?1", params![DOCUMENT_ID])?
        .into_iter().map(|row| ((row[0].clone(), row[1].clone()), row[2].clone())).collect::<HashMap<_, _>>();
    let existing_goals = db
        .query("SELECT data FROM goals", &[])?
        .into_iter()
        .filter_map(|row| serde_json::from_str::<Goal>(&row[0]).ok())
        .map(|goal| goal.title)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut new_goal_titles = HashSet::new();
    let mut totals = ImportTotals::default();
    let mut previews = Vec::new();
    let mut warnings = Vec::new();
    let mut pending = Vec::new();
    let mut supported_rows = 0;
    let mut archive_bytes = 0_usize;
    for sheet in &input.sheets {
        let mut item = ImportSheetPreview {
            name: sheet.name.clone(),
            target: None,
            rows: sheet.rows.len(),
            new: 0,
            updated: 0,
            skipped: 0,
            invalid: 0,
            columns: vec![],
            sample: vec![],
            warnings: vec![],
        };
        totals.rows += sheet.rows.len();
        let map = match mapping(sheet) {
            Ok(map) => map,
            Err(error) => {
                item.invalid = sheet.rows.len();
                item.warnings.push(warn(&sheet.name, None, error.1));
                totals.invalid += item.invalid;
                warnings.extend(item.warnings.clone());
                previews.push(item);
                continue;
            }
        };
        item.target = Some(map.kind.label().into());
        item.columns = map.columns.clone();
        if map.kind != TargetKind::Archive {
            supported_rows += sheet.rows.len();
        } else {
            item.warnings.push(warn(
                &sheet.name,
                None,
                "这张表仅保存原始列值，不创建目标或训练记录",
            ));
        }
        let missing = match map.kind {
            TargetKind::Goal => map.title.is_none(),
            TargetKind::Session => map.date.is_none() || map.observation.is_none(),
            TargetKind::Archive => false,
        };
        if missing {
            item.invalid = sheet.rows.len();
            totals.invalid += item.invalid;
            item.warnings
                .push(warn(&sheet.name, None, "缺少必需列，无法可靠导入这张表"));
            warnings.extend(item.warnings.clone());
            previews.push(item);
            continue;
        }
        if map.record_id.is_none() {
            item.warnings.push(warn(
                &sheet.name,
                None,
                "导出文件无来源记录 ID，本次按工作表行号去重；源表重排后需重新核对",
            ));
        }
        if map.kind == TargetKind::Goal && map.level.is_none() {
            item.warnings.push(warn(
                &sheet.name,
                None,
                "无层级列的阶段目标按历史短期目标导入，不推断上级",
            ));
        }
        for row in &sheet.rows {
            let key = if let Some(id_column) = map.record_id {
                let id = value(row, Some(id_column));
                if id.is_empty() {
                    item.invalid += 1;
                    item.warnings
                        .push(warn(&sheet.name, Some(row.row), "来源记录 ID 为空"));
                    continue;
                }
                format!("id:{id}")
            } else {
                format!("row:{}", row.row)
            };
            if !seen.insert((sheet.name.clone(), key.clone())) {
                item.invalid += 1;
                item.warnings
                    .push(warn(&sheet.name, Some(row.row), "来源记录 ID 重复"));
                continue;
            }
            let raw_json = serde_json::to_string(&(&sheet.headers, &row.cells))?;
            // Headers are retained with every source row. Bound their expanded
            // archive size as well as the compact, once-per-sheet input size.
            archive_bytes = archive_bytes.saturating_add(raw_json.len());
            if archive_bytes > MAX_SOURCE_ARCHIVE_BYTES {
                return Err(ApiError::invalid("原始资料存档内容较多，请分批导入"));
            }
            let raw_sha256 = sha(&raw_json);
            match existing.get(&(sheet.name.clone(), key.clone())) {
                Some(old) if old == &raw_sha256 => {
                    item.skipped += 1;
                    if item.sample.len() < 2 {
                        item.sample.push(ImportSample {
                            row: row.row,
                            label: sample_label(row, &map),
                            action: "已导入，跳过".into(),
                        });
                    }
                    continue;
                }
                Some(_) => {
                    item.invalid += 1;
                    item.warnings.push(warn(
                        &sheet.name,
                        Some(row.row),
                        "来源记录内容已变化；为避免覆盖现有修改，请人工核对后再导入",
                    ));
                    continue;
                }
                None => {}
            }
            let candidate = match map.kind {
                TargetKind::Goal => goal(row, &map).map(|goal| (Some(goal), None)),
                TargetKind::Session => session(row, &map).map(|session| (None, Some(session))),
                TargetKind::Archive => Ok((None, None)),
            };
            let (goal, session) = match candidate {
                Ok(pair) => pair,
                Err(error) => {
                    item.invalid += 1;
                    item.warnings
                        .push(warn(&sheet.name, Some(row.row), error.1));
                    continue;
                }
            };
            if let Some(ref goal) = goal
                && (existing_goals.contains(&goal.title)
                    || !new_goal_titles.insert(goal.title.clone()))
            {
                item.invalid += 1;
                item.warnings.push(warn(
                    &sheet.name,
                    Some(row.row),
                    "应用或本次文件中已有同名目标，请先核对是否为同一目标",
                ));
                continue;
            }
            if item.sample.len() < 2 {
                item.sample.push(ImportSample {
                    row: row.row,
                    label: sample_label(row, &map),
                    action: "新增".into(),
                });
            }
            item.new += 1;
            pending.push(PendingRow {
                sheet: sheet.name.clone(),
                row_key: key,
                raw_sha256,
                raw_json,
                kind: map.kind,
                goal,
                session,
            });
        }
        // A source can have many erroneous rows, but the response stays small.
        if item.warnings.len() > 100 {
            let count = item.warnings.len() - 100;
            item.warnings.truncate(100);
            item.warnings.push(warn(
                &sheet.name,
                None,
                format!("另有 {count} 条问题未逐行展示"),
            ));
        }
        totals.new += item.new;
        totals.skipped += item.skipped;
        totals.invalid += item.invalid;
        warnings.extend(item.warnings.iter().take(30).cloned());
        previews.push(item);
    }
    if supported_rows == 0 {
        warnings.push(warn("", None, "没有可映射的目标或训练记录；请检查导出内容"));
    }
    let preview = ImportPreview {
        preview_id: String::new(),
        filename: input.filename.clone(),
        expires_at: String::new(),
        can_commit: totals.invalid == 0 && totals.new > 0 && supported_rows > 0,
        totals,
        sheets: previews,
        warnings,
    };
    Ok((preview, pending))
}

fn sample_label(row: &ImportRow, map: &Mapping) -> String {
    let label = match map.kind {
        TargetKind::Goal => value(row, map.title),
        TargetKind::Session => value(row, map.date),
        TargetKind::Archive => "原始资料",
    };
    label.chars().take(60).collect()
}

pub fn preview(db: &dyn Sql, actor: &User, input: ImportPreviewInput) -> Result<ImportPreview> {
    ensure_admin(actor)?;
    if input.sheets.is_empty()
        || input.sheets.len() > 20
        || input.sheets.iter().map(|s| s.rows.len()).sum::<usize>() > 10_000
    {
        return Err(ApiError::invalid("文件工作表或记录数超过限制"));
    }
    let source_json = serde_json::to_string(&input)?;
    if source_json.len() > MAX_PREVIEW_BYTES {
        return Err(ApiError::invalid(
            "文件解析后的内容较多，请按工作表或日期分批导入（每批解析内容不超过 1 MB）",
        ));
    }
    let (mut summary, _) = analyze(db, &input)?;
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::seconds(PREVIEW_SECONDS);
    summary.preview_id = Uuid::new_v4().to_string();
    summary.expires_at = expires.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let summary_json = serde_json::to_string(&summary)?;
    if source_json.len() + summary_json.len() > 1_800_000 {
        return Err(ApiError::invalid("文件表头或预览内容较多，请分批导入"));
    }
    db.execute(
        "DELETE FROM import_previews WHERE expires_at<=?1",
        params![now.timestamp()],
    )?;
    db.execute("DELETE FROM import_previews WHERE id IN (SELECT id FROM import_previews WHERE owner_user_id=?1 ORDER BY expires_at DESC,id DESC LIMIT -1 OFFSET 2)", params![actor.id])?;
    db.execute("INSERT INTO import_previews(id,owner_user_id,filename,file_sha256,expires_at,data,summary) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![summary.preview_id, actor.id, input.filename, input.sha256, expires.timestamp(), source_json, summary_json])?;
    Ok(summary)
}

pub fn commit(db: &dyn Sql, actor: &User, input: ImportCommitInput) -> Result<ImportCommit> {
    ensure_admin(actor)?;
    if Uuid::parse_str(&input.preview_id).is_err() {
        return Err(ApiError::invalid("预览 ID 无效"));
    }
    if let Some(row) = db
        .query(
            "SELECT result FROM import_batches WHERE preview_id=?1",
            params![input.preview_id],
        )?
        .first()
    {
        return Ok(serde_json::from_str(&row[0])?);
    }
    let rows = db.query("SELECT owner_user_id,CAST(expires_at AS TEXT),data,summary FROM import_previews WHERE id=?1", params![input.preview_id])?;
    let row = rows
        .first()
        .ok_or_else(|| ApiError(StatusCode::GONE, "导入预览已失效，请重新上传预览".into()))?;
    if row[0] != actor.id || row[1].parse::<i64>().unwrap_or(0) <= chrono::Utc::now().timestamp() {
        return Err(ApiError(
            StatusCode::GONE,
            "导入预览已失效，请重新上传预览".into(),
        ));
    }
    let source: ImportPreviewInput = serde_json::from_str(&row[2])?;
    let previous: ImportPreview = serde_json::from_str(&row[3])?;
    let (current, pending) = analyze(db, &source)?;
    if !previous.can_commit || !current.can_commit {
        return Err(ApiError::invalid("导入预览包含未解决的问题，不能提交"));
    }
    if current.totals.new != previous.totals.new
        || current.totals.skipped != previous.totals.skipped
        || current.totals.invalid != previous.totals.invalid
    {
        return Err(ApiError::conflict("数据已变化，请重新上传并核对预览"));
    }
    let batch_id = Uuid::new_v4().to_string();
    let result = ImportCommit {
        batch_id: batch_id.clone(),
        totals: current.totals,
        sheets: current.sheets,
        warnings: current.warnings,
    };
    db.execute("INSERT INTO import_batches(id,preview_id,filename,file_sha256,committed_at,result) VALUES (?1,?2,?3,?4,?5,?6)",
        params![batch_id, input.preview_id, source.filename, source.sha256, chrono::Utc::now().to_rfc3339(), serde_json::to_string(&result)?])?;
    for entry in pending {
        let target_id = match (entry.goal, entry.session) {
            (Some(goal), None) => {
                db.execute(
                    "INSERT INTO goals(id,parent_id,data) VALUES (?1,NULL,?2)",
                    params![goal.id, serde_json::to_string(&goal)?],
                )?;
                store::record_goal_revision(db, &goal, "腾讯文档历史导入", None)?;
                Some(goal.id)
            }
            (None, Some(session)) => {
                db.execute(
                    "INSERT INTO sessions(id,data) VALUES (?1,?2)",
                    params![session.id, serde_json::to_string(&session)?],
                )?;
                Some(session.id)
            }
            (None, None) => None,
            _ => return Err(ApiError::storage()),
        };
        db.execute("INSERT INTO import_source_rows(source_document,source_sheet,source_row_key,raw_sha256,raw_json,target_kind,target_id,batch_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![DOCUMENT_ID, entry.sheet, entry.row_key, entry.raw_sha256, entry.raw_json, entry.kind.key(), target_id, batch_id])?;
    }
    db.execute(
        "DELETE FROM import_previews WHERE id=?1",
        params![input.preview_id],
    )?;
    Ok(result)
}
