//! Human-readable current data and lossless, UTF-16-safe raw JSON chunks.
use rust_xlsxwriter::{DocProperties, Format, Workbook, XlsxError};
use serde_json::Value;
use skarma_core::{
    foundation::export::Snapshot,
    store::{ApiError, Result},
};
use std::collections::{BTreeMap, BTreeSet};
fn flatten(value: &Value, prefix: &str, depth: u8, out: &mut BTreeMap<String, Value>) {
    if let Value::Object(map) = value
        && depth < 2
    {
        for (k, v) in map {
            flatten(
                v,
                &if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                },
                depth + 1,
                out,
            );
        }
        return;
    }
    out.insert(prefix.into(), value.clone());
}
fn label(path: &str) -> String {
    path.split('.')
        .map(|v| match v {
            "input" => "填写",
            "id" => "业务ID",
            "title" => "名称",
            "name" => "名称",
            "version" => "版本",
            "status" => "状态",
            "business_date" => "业务日期",
            "course" => "课程",
            "teacher" => "老师",
            "notes" => "观察",
            "teacher_notes" => "老师建议",
            "goal_id" => "目标ID",
            "method_id" => "方法ID",
            "source_lesson_id" => "来源课记ID",
            "source_session_id" => "来源干预ID",
            "content" => "内容",
            "created_at" => "创建时间",
            "creator_name" => "记录者",
            "deleted" => "删除标记",
            "audit" => "修订",
            "actor_name" => "修订者",
            "note" => "说明",
            "at" => "服务器时间",
            "criterion" => "指标标准",
            "criteria" => "观察标准",
            "domain" => "领域",
            "due" => "目标日期",
            "support" => "帮助条件",
            "opportunities" => "机会数",
            "achieved" => "做到数",
            "amount" => "实际值",
            "text" => "实际观察",
            "phase" => "计划或实际",
            "role" => "目标角色",
            "meaning" => "生活意义",
            "parent_id" => "上级ID",
            "level" => "目标层级",
            "archived" => "收起",
            "plan" => "安排",
            "time_slot" => "时段",
            "description" => "做法",
            "template" => "观察模板",
            "state" => "推进状态",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
fn chunks(text: &str) -> Vec<String> {
    let mut result = vec![];
    let mut part = String::new();
    let mut units = 0;
    for ch in text.chars() {
        if units + ch.len_utf16() > 30000 {
            result.push(std::mem::take(&mut part));
            units = 0;
        }
        part.push(ch);
        units += ch.len_utf16();
    }
    result.push(part);
    result
}
fn write(snapshot: Snapshot) -> std::result::Result<Vec<u8>, XlsxError> {
    let mut book = Workbook::new();
    book.set_properties(
        &DocProperties::new()
            .set_author("skarmar")
            .set_title("skarmar业务数据导出"),
    );
    let header = Format::new()
        .set_bold()
        .set_background_color("#34765B")
        .set_font_color("#FFFFFF")
        .set_text_wrap();
    let text = Format::new().set_text_wrap();
    let info = book.add_worksheet();
    info.set_name("导出说明")?;
    info.set_column_width(0, 28)?;
    info.set_column_width(1, 100)?;
    for (i,(key,value)) in [("格式版本",snapshot.schema.as_str()),("新模型数据库版本",&snapshot.database_version.to_string()),("导出时间（ISO 8601）",snapshot.exported_at.as_str()),("业务日期时区","Asia/Shanghai；日期按已保存值保留"),("范围","包含已保存的计划和总结草稿、删除标记、历史及来源快照；不含本机未提交内容、待上传队列、凭据或会话"),("统计口径","只有正式完成的干预计次数；未撤销或删除的实际课记计节数。计划和实际方法分别列出"),("长内容","超出Excel单元格长度的内容在主表提示，完整原始业务JSON保存在原始数据分段，可按业务表、记录序号、片段顺序重组"),("使用边界","这是业务数据导出，不是可直接恢复的数据库备份，也不承诺兼容旧版工作簿导入")].into_iter().enumerate(){info.write_string(i as u32,0,key)?;info.write_string_with_format(i as u32,1,value,&text)?;}
    type RawRow = (String, u32, String, u64, usize, usize, String);
    let mut raw_rows: Vec<RawRow> = vec![];
    let mut counts = vec![];
    for (name, records) in snapshot.tables {
        counts.push((name.clone(), records.len()));
        let rows = records
            .iter()
            .map(|r| {
                let mut out = BTreeMap::new();
                flatten(r, "", 0, &mut out);
                out
            })
            .collect::<Vec<_>>();
        let columns = rows
            .iter()
            .flat_map(|r| r.keys().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let sheet = book.add_worksheet();
        sheet.set_name(&name)?;
        sheet.set_freeze_panes(1, 0)?;
        sheet.set_default_row_height(42);
        sheet.set_column_range_width(0, columns.len().max(1) as u16 - 1, 26)?;
        if columns.is_empty() {
            sheet.write_string_with_format(0, 0, "暂无已保存记录", &header)?;
        } else {
            for (c, key) in columns.iter().enumerate() {
                sheet.write_string_with_format(0, c as u16, label(key), &header)?;
            }
            for (r, row) in rows.iter().enumerate() {
                for (c, key) in columns.iter().enumerate() {
                    let Some(v) = row.get(key) else {
                        continue;
                    };
                    let r = u32::try_from(r + 1).map_err(|_| XlsxError::RowColumnLimitError)?;
                    let c = c as u16;
                    match v {
                        Value::Null => {}
                        Value::Bool(v) => {
                            sheet.write_boolean(r, c, *v)?;
                        }
                        Value::Number(n) => {
                            if let Some(n) = n.as_f64().filter(|n| n.abs() <= 9007199254740991.0) {
                                sheet.write_number(r, c, n)?;
                            } else {
                                sheet.write_string(r, c, n.to_string())?;
                            }
                        }
                        other => {
                            let value = other
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| other.to_string());
                            let value = if value.encode_utf16().count() > 32767 {
                                "长内容请查看原始数据分段"
                            } else {
                                &value
                            };
                            sheet.write_string_with_format(r, c, value, &text)?;
                        }
                    }
                }
            }
            sheet.autofilter(0, 0, rows.len() as u32, columns.len() as u16 - 1)?;
        }
        for (row, value) in records.iter().enumerate() {
            let json = value.to_string();
            let parts = chunks(&json);
            let id = value["id"]
                .as_str()
                .or_else(|| value["input"]["id"].as_str())
                .unwrap_or("");
            for (i, part) in parts.iter().enumerate() {
                raw_rows.push((
                    name.clone(),
                    row as u32 + 1,
                    id.into(),
                    value["version"].as_u64().unwrap_or(0),
                    i + 1,
                    parts.len(),
                    part.clone(),
                ));
            }
        }
    }
    let counts_sheet = book.add_worksheet();
    counts_sheet.set_name("表记录数")?;
    counts_sheet.write_string_with_format(0, 0, "业务表", &header)?;
    counts_sheet.write_string_with_format(0, 1, "导出记录数（含状态标记）", &header)?;
    for (i, (name, count)) in counts.into_iter().enumerate() {
        counts_sheet.write_string(i as u32 + 1, 0, name)?;
        counts_sheet.write_number(i as u32 + 1, 1, count as f64)?;
    }
    let raw = book.add_worksheet();
    raw.set_name("原始数据分段")?;
    raw.set_freeze_panes(1, 0)?;
    for (c, h) in [
        "业务表",
        "记录序号",
        "业务ID",
        "版本",
        "片段",
        "总片段",
        "完整业务JSON分段",
    ]
    .iter()
    .enumerate()
    {
        raw.write_string_with_format(0, c as u16, *h, &header)?;
    }
    raw.set_column_width(6, 100)?;
    for (i, (name, index, id, version, part, total, json)) in raw_rows.into_iter().enumerate() {
        let row = u32::try_from(i + 1).map_err(|_| XlsxError::RowColumnLimitError)?;
        raw.write_string(row, 0, name)?;
        raw.write_number(row, 1, index)?;
        raw.write_string(row, 2, id)?;
        raw.write_number(row, 3, version as f64)?;
        raw.write_number(row, 4, part as f64)?;
        raw.write_number(row, 5, total as f64)?;
        raw.write_string_with_format(row, 6, json, &text)?;
    }
    book.save_to_buffer()
}
pub fn workbook(snapshot: Snapshot) -> Result<Vec<u8>> {
    if snapshot.schema != "skarmar.prd.export.v1" {
        return Err(ApiError::invalid("导出格式版本不匹配"));
    }
    write(snapshot)
        .map_err(|_| ApiError::invalid("数据超过Excel容量或导出暂未完成，请稍后重试或分批导出"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Data, Reader, Xlsx};
    use std::io::Cursor;
    #[test]
    fn export_keeps_numbers_literal_formula_text_and_full_unicode_chunks() {
        let long = "🙂".repeat(20000);
        let row = serde_json::json!({"id":"synthetic","version":1,"input":{"opportunities":5,"achieved":3,"text":long,"name":"=SUM(A1:A5)"}});
        let tables = BTreeMap::from([("实际测量".into(), vec![row.clone()])]);
        let bytes = workbook(Snapshot {
            schema: "skarmar.prd.export.v1".into(),
            database_version: 5,
            exported_at: "2026-10-24T00:00:00Z".into(),
            tables,
        })
        .unwrap();
        let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).unwrap();
        let sheet = book.worksheet_range("实际测量").unwrap();
        let rows = sheet.rows().collect::<Vec<_>>();
        let opportunities = rows[0]
            .iter()
            .position(|v| matches!(v,Data::String(s)if s=="填写 · 机会数"))
            .unwrap();
        assert!(matches!(
            rows[1][opportunities],
            Data::Float(5.0) | Data::Int(5)
        ));
        assert!(
            rows[1]
                .iter()
                .any(|v| matches!(v,Data::String(s)if s=="=SUM(A1:A5)"))
        );
        let raw = book.worksheet_range("原始数据分段").unwrap();
        let restored = raw
            .rows()
            .skip(1)
            .map(|r| r[6].to_string())
            .collect::<String>();
        assert_eq!(serde_json::from_str::<Value>(&restored).unwrap(), row);
        assert!(
            raw.rows()
                .skip(1)
                .all(|r| r[6].to_string().encode_utf16().count() <= 30000)
        );
        assert!(book.worksheet_formula("实际测量").unwrap().is_empty());
    }
}
