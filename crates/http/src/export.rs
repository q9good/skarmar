//! The same in-memory XLSX writer runs on the host and inside the SQLite Durable Object.
use axum::http::StatusCode;
use rust_xlsxwriter::{DocProperties, Format, FormatAlign, Workbook, XlsxError};
use skarma_core::{
    model::*,
    store::{ApiError, Result},
};
use std::collections::HashMap;

pub const MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const DISPOSITION: &str = "attachment; filename=\"skarma-export.xlsx\"";

const GOAL_HEADERS: &[&str] = &[
    "目标ID",
    "目标名称",
    "层级",
    "上级目标ID",
    "领域",
    "达成标准",
    "归档状态",
    "版本",
    "培养／训练状态",
    "开始日期",
    "截止日期",
    "上次复盘日期",
    "下次复盘日期",
    "达成确认日期",
    "复盘备注",
];

fn optional(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}
fn goal_row(goal: &Goal) -> Vec<String> {
    vec![
        goal.id.clone(),
        goal.title.clone(),
        match goal.level {
            Level::Long => "长期",
            Level::Medium => "中期",
            Level::Short => "短期",
        }
        .into(),
        optional(&goal.parent_id),
        goal.area.clone(),
        goal.criteria.clone(),
        match goal.status {
            GoalStatus::Active => "活动",
            GoalStatus::Archived => "已归档",
        }
        .into(),
        goal.version.to_string(),
        match goal.progress_status {
            Some(GoalProgress::Cultivating) => "持续培养中",
            Some(GoalProgress::Internalized) => "已内化",
            Some(GoalProgress::NotStarted) => "未开始",
            Some(GoalProgress::InProgress) => "进行中",
            Some(GoalProgress::Achieved) => "已达成",
            Some(GoalProgress::Completed) => "已完成",
            Some(GoalProgress::Overdue) => "已逾期（历史状态）",
            None => "",
        }
        .into(),
        optional(&goal.start_date),
        optional(&goal.due_date),
        optional(&goal.last_review_date),
        optional(&goal.next_review_date),
        optional(&goal.completion_date),
        goal.review_notes.clone(),
    ]
}
fn session_status(value: &str) -> &str {
    match value {
        "draft" => "已保存草稿",
        "completed" => "已完成",
        value => value,
    }
}
fn experience_kind(value: &str) -> &str {
    match value {
        "general" => "长期普适",
        "task" => "中短期任务",
        value => value,
    }
}

fn sheet(
    book: &mut Workbook,
    name: &str,
    headers: &[&str],
    rows: impl IntoIterator<Item = Vec<String>>,
) -> std::result::Result<(), XlsxError> {
    let worksheet = book.add_worksheet();
    worksheet.set_name(name)?;
    worksheet.set_freeze_panes(1, 0)?;
    worksheet.set_column_range_width(0, headers.len() as u16 - 1, 26)?;
    worksheet.set_column_width(0, 40)?;
    worksheet.set_row_height(0, 32)?;
    worksheet.set_default_row_height(48);
    let header = Format::new()
        .set_bold()
        .set_font_color("#FFFFFF")
        .set_background_color("#34765B")
        .set_text_wrap()
        .set_align(FormatAlign::VerticalCenter);
    let text = Format::new().set_text_wrap().set_align(FormatAlign::Top);
    for (column, title) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, column as u16, *title, &header)?;
    }
    let mut last_row = 0;
    for (index, cells) in rows.into_iter().enumerate() {
        let row = u32::try_from(index + 1).map_err(|_| XlsxError::RowColumnLimitError)?;
        for (column, value) in cells.into_iter().enumerate() {
            if value.encode_utf16().count() > 32_767 {
                return Err(XlsxError::MaxStringLengthExceeded);
            }
            // Explicit strings preserve dates/IDs, and never turn user text into a formula or link.
            worksheet.write_string_with_format(row, column as u16, &value, &text)?;
        }
        last_row = row;
    }
    worksheet.autofilter(0, 0, last_row, headers.len() as u16 - 1)?;
    Ok(())
}

fn write(snapshot: ExportSnapshot) -> std::result::Result<Vec<u8>, XlsxError> {
    let mut book = Workbook::new();
    book.set_properties(
        &DocProperties::new()
            .set_author("skarma")
            .set_title("训练记录导出"),
    );
    let state = &snapshot.state;
    let source_dates: HashMap<_, _> = state
        .sessions
        .iter()
        .map(|session| (session.id.as_str(), session.input.business_date.as_str()))
        .collect();
    sheet(&mut book, "说明", &["项目", "说明"], [
        vec!["导出时间（UTC）".into(), snapshot.exported_at.clone()],
        vec!["范围".into(), "当前空间中已保存的全部目标、途径、训练草稿、完成记录、上课记录、困难、经验、目标历史及显式关联。浏览器中尚未保存的草稿不在文件内。".into()],
        vec!["历史快照".into(), "逐目标明细中的「当时」字段来自训练保存的目标快照。快照缺失时留空并标记，不使用当前目标覆盖历史。".into()],
        vec!["空白值".into(), "空白表示未填写或未知，不代表零分或已完成。日期、ID、版本及正文按文本保留。".into()],
        vec!["记录关联".into(), "通过目标ID、上级目标ID、途径ID、经验ID、训练ID及来源训练ID关联各表。目的途径关系与经验关联反映当前显式绑定；目标历史保留已记录版本。".into()],
        vec!["活动列表".into(), "训练活动以单元格内换行分隔，每行一个活动。".into()],
        vec!["用途".into(), "文件可用 Excel / WPS 打开、筛选和归档。恢复系统仍使用数据库备份；当前不支持 Excel 导入。".into()],
        vec!["目标／途径／训练／困难／经验／历史版本数量".into(), format!("{} / {} / {} / {} / {} / {}", state.goals.len(), state.pathways.len(), state.sessions.len(), state.difficulties.len(), state.experiences.len(), snapshot.goal_history.len())],
    ])?;
    sheet(
        &mut book,
        "目标",
        GOAL_HEADERS,
        state.goals.iter().map(goal_row),
    )?;
    sheet(
        &mut book,
        "训练记录",
        &[
            "训练ID",
            "版本",
            "训练状态",
            "训练日期",
            "时段",
            "训练活动",
            "组织方式",
            "整次训练安排",
            "整次观察",
            "目标数量",
            "有困难",
            "困难详情",
            "有经验",
            "经验详情",
            "经验类型",
        ],
        state.sessions.iter().map(|session| {
            let input = &session.input;
            vec![
                session.id.clone(),
                session.version.to_string(),
                session_status(&session.status).into(),
                input.business_date.clone(),
                input.time_slot.clone(),
                input.activities.join("\n"),
                input.organization.clone(),
                input.plan.clone(),
                input.observation.clone(),
                input.targets.len().to_string(),
                if input.has_difficulty { "是" } else { "否" }.into(),
                input.difficulty.clone(),
                if input.has_experience { "是" } else { "否" }.into(),
                input.experience.clone(),
                experience_kind(&input.experience_kind).into(),
            ]
        }),
    )?;
    sheet(
        &mut book,
        "上课记录",
        &[
            "记录ID",
            "版本",
            "上课日期",
            "课程名称",
            "授课老师",
            "上课内容",
            "上课目标",
            "存在问题",
            "家里需配合",
        ],
        state.lessons.iter().map(|lesson| {
            vec![
                lesson.id.clone(),
                lesson.version.to_string(),
                lesson.input.business_date.clone(),
                lesson.input.course.clone(),
                lesson.input.teacher.clone(),
                lesson.input.content.clone(),
                lesson.input.goals.clone(),
                lesson.input.problems.clone(),
                lesson.input.home_cooperation.clone(),
            ]
        }),
    )?;
    let mut target_headers = vec![
        "训练ID",
        "训练版本",
        "训练状态",
        "训练日期",
        "时段",
        "目标ID",
        "目标角色",
        "逐目标安排",
        "实际进度",
        "回填结论",
        "下一步",
        "快照情况",
    ];
    let snapshot_headers: Vec<_> = GOAL_HEADERS
        .iter()
        .map(|header| format!("当时{header}"))
        .collect();
    target_headers.extend(snapshot_headers.iter().map(String::as_str));
    sheet(
        &mut book,
        "逐目标明细",
        &target_headers,
        state.sessions.iter().flat_map(|session| {
            session.input.targets.iter().map(move |target| {
                let goal = session
                    .target_snapshots
                    .iter()
                    .find(|saved| {
                        saved.input.goal_id == target.goal_id && saved.input.role == target.role
                    })
                    .map(|saved| &saved.goal_snapshot);
                let mut cells = vec![
                    session.id.clone(),
                    session.version.to_string(),
                    session_status(&session.status).into(),
                    session.input.business_date.clone(),
                    session.input.time_slot.clone(),
                    target.goal_id.clone(),
                    match target.role {
                        Role::Primary => "主要",
                        Role::Secondary => "次要",
                        Role::Review => "复习",
                    }
                    .into(),
                    target.plan.clone(),
                    target.progress.clone(),
                    match target.outcome {
                        Some(Outcome::Continue) => "继续",
                        Some(Outcome::Archive) => "归档",
                        Some(Outcome::Adjust) => "调整",
                        None => "",
                    }
                    .into(),
                    target.next_step.clone(),
                    if goal.is_some() {
                        "已保存"
                    } else {
                        "缺失，需核对"
                    }
                    .into(),
                ];
                cells.extend(
                    goal.map(goal_row)
                        .unwrap_or_else(|| vec![String::new(); GOAL_HEADERS.len()]),
                );
                cells
            })
        }),
    )?;
    for (name, items) in [("困难", &state.difficulties), ("经验", &state.experiences)] {
        sheet(
            &mut book,
            name,
            &[
                "记录ID",
                "版本",
                "来源训练ID",
                "来源训练日期",
                "内容",
                "经验类型",
                "跟进状态",
                "下次跟进日期",
                "跟进结论／验证依据",
            ],
            items.iter().map(|item| {
                vec![
                    item.id.clone(),
                    item.version.to_string(),
                    optional(&item.source_session_id),
                    item.source_session_id
                        .as_deref()
                        .and_then(|id| source_dates.get(id))
                        .copied()
                        .unwrap_or_default()
                        .into(),
                    item.content.clone(),
                    experience_kind(item.kind.as_deref().unwrap_or_default()).into(),
                    match item.status.as_str() {
                        "pending" if name == "困难" => "待确认",
                        "pending" => "待验证",
                        "active" => "跟进中",
                        "resolved" => "已解决",
                        "verified" => "已验证",
                        "stopped" => "停用",
                        value => value,
                    }
                    .into(),
                    optional(&item.next_review_date),
                    item.conclusion.clone(),
                ]
            }),
        )?;
    }
    let mut history_headers = GOAL_HEADERS.to_vec();
    history_headers.extend(["版本记录时间（UTC）", "修改说明", "来源训练ID"]);
    sheet(
        &mut book,
        "目标历史",
        &history_headers,
        snapshot.goal_history.iter().map(|revision| {
            let mut cells = goal_row(&revision.goal);
            cells.extend([
                revision.recorded_at.clone(),
                revision.change_note.clone(),
                optional(&revision.source_session_id),
            ]);
            cells
        }),
    )?;
    sheet(
        &mut book,
        "途径",
        &["途径ID", "版本", "途径名称", "作用说明", "具体步骤", "状态"],
        state.pathways.iter().map(|pathway| {
            vec![
                pathway.id.clone(),
                pathway.version.to_string(),
                pathway.title.clone(),
                pathway.description.clone(),
                pathway.steps.clone(),
                match pathway.status {
                    PathwayStatus::Active => "活动",
                    PathwayStatus::Archived => "已归档",
                }
                .into(),
            ]
        }),
    )?;
    let goals: HashMap<_, _> = state
        .goals
        .iter()
        .map(|goal| (goal.id.as_str(), goal))
        .collect();
    let pathways: HashMap<_, _> = state
        .pathways
        .iter()
        .map(|pathway| (pathway.id.as_str(), pathway))
        .collect();
    sheet(
        &mut book,
        "目的途径关系",
        &["目的ID", "目的名称", "途径ID", "途径名称"],
        state.pathways.iter().flat_map(|pathway| {
            let goals = &goals;
            pathway.goal_ids.iter().map(move |goal_id| {
                vec![
                    goal_id.clone(),
                    goals
                        .get(goal_id.as_str())
                        .map(|goal| goal.title.clone())
                        .unwrap_or_default(),
                    pathway.id.clone(),
                    pathway.title.clone(),
                ]
            })
        }),
    )?;
    sheet(
        &mut book,
        "经验关联",
        &["经验ID", "经验版本", "关联类型", "关联ID", "关联名称"],
        state.experiences.iter().flat_map(|item| {
            let goals = &goals;
            let pathways = &pathways;
            item.goal_ids
                .iter()
                .map(move |id| {
                    vec![
                        item.id.clone(),
                        item.version.to_string(),
                        "目的".into(),
                        id.clone(),
                        goals
                            .get(id.as_str())
                            .map(|goal| goal.title.clone())
                            .unwrap_or_default(),
                    ]
                })
                .chain(item.pathway_ids.iter().map(move |id| {
                    vec![
                        item.id.clone(),
                        item.version.to_string(),
                        "途径".into(),
                        id.clone(),
                        pathways
                            .get(id.as_str())
                            .map(|pathway| pathway.title.clone())
                            .unwrap_or_default(),
                    ]
                }))
        }),
    )?;
    book.save_to_buffer()
}

pub fn workbook(snapshot: ExportSnapshot) -> Result<Vec<u8>> {
    write(snapshot).map_err(|error| match error {
        XlsxError::RowColumnLimitError | XlsxError::MaxStringLengthExceeded => {
            ApiError::invalid("数据超过 Excel 的单表行数或单元格长度限制，请联系管理员分批导出")
        }
        _ => ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Excel 导出失败，请稍后重试".into(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> ExportSnapshot {
        ExportSnapshot {
            exported_at: "2026-10-13T00:00:00Z".into(),
            state: AppState {
                goals: vec![],
                sessions: vec![],
                difficulties: vec![],
                experiences: vec![],
                pathways: vec![],
                lessons: vec![],
            },
            goal_history: vec![],
        }
    }

    #[test]
    fn an_empty_space_still_produces_a_workbook() {
        assert!(workbook(empty()).unwrap().starts_with(b"PK\x03\x04"));
    }

    #[test]
    fn an_overlong_cell_is_rejected_without_silently_truncating_unicode() {
        let mut snapshot = empty();
        snapshot.state.difficulties.push(FollowUp {
            id: "test".into(),
            version: 1,
            source_session_id: None,
            content: "😀".repeat(16_384),
            kind: None,
            status: "pending".into(),
            next_review_date: None,
            conclusion: String::new(),
            goal_ids: vec![],
            pathway_ids: vec![],
        });
        assert_eq!(
            workbook(snapshot).unwrap_err().0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
