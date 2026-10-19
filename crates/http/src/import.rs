use std::{collections::BTreeMap, io::Cursor};

use calamine::{Data, DataRef, Reader, Xlsx};
use sha2::{Digest, Sha256};
use skarma_core::{
    Command,
    import::{ImportPreviewInput, ImportRow, ImportSheet},
    store::{ApiError, Result},
};

/// Only a verified import-authorize command can become a preview command.
pub fn preview_command(authorization: Command, input: ImportPreviewInput) -> Result<Command> {
    match authorization {
        Command::Authenticated { token, command }
            if matches!(*command, Command::ImportAuthorize) =>
        {
            Ok(Command::Authenticated {
                token,
                command: Box::new(Command::ImportPreview(input)),
            })
        }
        _ => Err(skarma_core::auth::unauthorized()),
    }
}

pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SHEETS: usize = 20;
const MAX_ROWS: usize = 10_000;
const MAX_COLUMNS: usize = 100;
const MAX_CELL_CHARS: usize = 20_000;
const MAX_TOTAL_CHARS: usize = 2_000_000;

fn clean_filename(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty()
        || name.len() > 200
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(ApiError::invalid("请选择名称有效的 Excel 或 CSV 文件"));
    }
    Ok(name.into())
}

fn checked_sheet(name: String, rows: Vec<(usize, Vec<String>)>) -> Result<ImportSheet> {
    let Some((header_row, headers)) = rows
        .iter()
        .find(|(_, row)| row.iter().any(|v| !v.is_empty()))
    else {
        return Err(ApiError::invalid("工作表没有表头"));
    };
    if headers.len() > MAX_COLUMNS || headers.is_empty() {
        return Err(ApiError::invalid("工作表列数超过限制"));
    }
    let header_row = *header_row;
    let headers = headers.clone();
    let data = rows
        .into_iter()
        .filter(|(row, cells)| *row > header_row && cells.iter().any(|v| !v.is_empty()))
        .map(|(row, cells)| ImportRow { row, cells })
        .collect::<Vec<_>>();
    if data.len() > MAX_ROWS {
        return Err(ApiError::invalid("文件记录数超过 10000 行限制，请分批导入"));
    }
    Ok(ImportSheet {
        name,
        headers,
        rows: data,
    })
}

fn parse_csv(filename: &str, bytes: &[u8]) -> Result<Vec<ImportSheet>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);
    let mut rows = Vec::new();
    let mut total_chars = 0;
    for (index, record) in reader.records().enumerate() {
        if index > MAX_ROWS {
            return Err(ApiError::invalid("文件记录数超过 10000 行限制，请分批导入"));
        }
        let record =
            record.map_err(|_| ApiError::invalid("CSV 无法读取；请导出 UTF-8 CSV 或 Excel"))?;
        if record.len() > MAX_COLUMNS {
            return Err(ApiError::invalid("CSV 列数超过限制"));
        }
        let cells = record
            .iter()
            .map(|value| value.trim_start_matches('\u{feff}').to_owned())
            .collect::<Vec<_>>();
        total_chars += cells
            .iter()
            .map(|value| value.chars().count())
            .sum::<usize>();
        if total_chars > MAX_TOTAL_CHARS
            || cells
                .iter()
                .any(|value| value.chars().count() > MAX_CELL_CHARS)
        {
            return Err(ApiError::invalid("文件正文超过导入限制"));
        }
        rows.push((index + 1, cells));
    }
    let name = filename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(filename)
        .to_owned();
    Ok(vec![checked_sheet(name, rows)?])
}

fn inspect_zip(bytes: &[u8]) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| ApiError::invalid("Excel 文件损坏或不是 .xlsx 格式"))?;
    if archive.len() > 1_000 {
        return Err(ApiError::invalid("Excel 文件包含过多内部文件"));
    }
    let mut uncompressed = 0_u64;
    for index in 0..archive.len() {
        let item = archive
            .by_index(index)
            .map_err(|_| ApiError::invalid("Excel 文件无法读取"))?;
        uncompressed = uncompressed.saturating_add(item.size());
        if item.size() > MAX_UNCOMPRESSED_BYTES || uncompressed > MAX_UNCOMPRESSED_BYTES {
            return Err(ApiError::invalid("Excel 解压后超过 64 MiB 限制"));
        }
    }
    Ok(())
}

fn cell_text(value: &DataRef<'_>) -> String {
    // Preserve the workbook's date epoch. Rendering a date as its raw serial
    // would make the core's legacy 1900-epoch fallback shift 1904 workbooks.
    if let DataRef::DateTime(value) = value
        && value.is_datetime()
    {
        let (year, month, day, hour, minute, second, millis) = value.to_ymd_hms_milli();
        return format!(
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}"
        );
    }
    Data::from(value.clone()).to_string()
}

fn parse_xlsx(bytes: &[u8]) -> Result<Vec<ImportSheet>> {
    inspect_zip(bytes)?;
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes))
        .map_err(|_| ApiError::invalid("Excel 文件损坏或不是 .xlsx 格式"))?;
    let names = workbook.sheet_names().to_vec();
    if names.is_empty() || names.len() > MAX_SHEETS {
        return Err(ApiError::invalid("Excel 工作表数量超过限制，或没有工作表"));
    }
    let mut sheets = Vec::new();
    let mut total_rows = 0;
    let mut total_chars = 0;
    for name in names {
        // worksheet_range() allocates a dense rectangle before callers can
        // inspect it. Read sparse cells directly so distant coordinates in a
        // tiny workbook cannot allocate millions of empty cells in the DO.
        let mut reader = workbook
            .worksheet_cells_reader(&name)
            .map_err(|_| ApiError::invalid("Excel 工作表无法读取"))?;
        let mut sparse_rows: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();
        let (mut first_row, mut last_row, mut first_col, mut last_col) = (u32::MAX, 0, u32::MAX, 0);
        while let Some(cell) = reader
            .next_cell()
            .map_err(|_| ApiError::invalid("Excel 工作表无法读取"))?
        {
            if matches!(cell.get_value(), DataRef::Empty) {
                continue;
            }
            let (row, col) = cell.get_position();
            first_row = first_row.min(row);
            last_row = last_row.max(row);
            first_col = first_col.min(col);
            last_col = last_col.max(col);
            let width = u64::from(last_col) - u64::from(first_col) + 1;
            let height = u64::from(last_row) - u64::from(first_row) + 1;
            if width > MAX_COLUMNS as u64 {
                return Err(ApiError::invalid("Excel 工作表列数超过限制"));
            }
            if height > (MAX_ROWS + MAX_SHEETS) as u64 {
                return Err(ApiError::invalid("文件记录数超过 10000 行限制，请分批导入"));
            }
            let value = cell_text(cell.get_value());
            let chars = value.chars().count();
            total_chars += chars;
            if total_chars > MAX_TOTAL_CHARS || chars > MAX_CELL_CHARS {
                return Err(ApiError::invalid("文件正文超过导入限制"));
            }
            let cells = sparse_rows.entry(row).or_default();
            // Duplicate cell references are not a way to bypass bounded
            // storage, even when they contain empty strings.
            if cells.len() >= MAX_COLUMNS {
                return Err(ApiError::invalid("Excel 工作表单行单元格数超过限制"));
            }
            cells.push((col, value));
        }
        if sparse_rows.is_empty() {
            continue;
        }
        total_rows += (u64::from(last_row) - u64::from(first_row) + 1) as usize;
        if total_rows > MAX_ROWS + MAX_SHEETS {
            return Err(ApiError::invalid("文件记录数超过 10000 行限制，请分批导入"));
        }
        let width = (last_col - first_col + 1) as usize;
        let rows = sparse_rows
            .into_iter()
            .map(|(row, cells)| {
                let mut values = vec![String::new(); width];
                for (col, value) in cells {
                    values[(col - first_col) as usize] = value;
                }
                // Empty intervening rows need no allocation: checked_sheet
                // discards them, while each retained row keeps its source ID.
                (row as usize + 1, values)
            })
            .collect();
        sheets.push(checked_sheet(name, rows)?);
    }
    if sheets.is_empty() {
        return Err(ApiError::invalid("Excel 没有可导入的工作表"));
    }
    Ok(sheets)
}

pub fn parse(filename: &str, bytes: &[u8]) -> Result<ImportPreviewInput> {
    let filename = clean_filename(filename)?;
    if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
        return Err(ApiError::invalid(
            "请选择不超过 10 MiB 的 Excel 或 CSV 文件",
        ));
    }
    let extension = filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    let sheets = match extension.as_deref() {
        Some("xlsx") => parse_xlsx(bytes)?,
        Some("csv") => parse_csv(&filename, bytes)?,
        _ => return Err(ApiError::invalid("只支持 .xlsx 或 .csv 文件")),
    };
    if sheets.iter().all(|sheet| sheet.rows.is_empty()) {
        return Err(ApiError::invalid("文件没有可导入的记录"));
    }
    Ok(ImportPreviewInput {
        filename,
        sha256: format!("{:x}", Sha256::digest(bytes)),
        sheets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utf8_csv_with_quoted_newline() {
        let parsed = parse(
            "每日训练记录.csv",
            "日期,训练记录\n2026-10-17,\"第一行\n第二行\"\n".as_bytes(),
        )
        .unwrap();
        assert_eq!(parsed.sheets[0].name, "每日训练记录");
        assert_eq!(parsed.sheets[0].rows[0].cells[1], "第一行\n第二行");
    }

    #[test]
    fn parses_real_xlsx() {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name("阶段目标").unwrap();
        sheet.write_string(0, 0, "阶段目标名称").unwrap();
        sheet.write_string(1, 0, "主动表达需求").unwrap();
        let bytes = workbook.save_to_buffer().unwrap();
        let parsed = parse("个训记录.xlsx", &bytes).unwrap();
        assert_eq!(parsed.sheets[0].rows[0].cells[0], "主动表达需求");
    }

    #[test]
    fn rejects_sparse_xlsx_extent_before_dense_allocation() {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name("阶段目标").unwrap();
        sheet.write_string(0, 0, "阶段目标名称").unwrap();
        // Only two populated cells, but a dense range would contain one
        // hundred million values and exhaust a Durable Object's memory.
        sheet.write_string(999_999, 99, "合成远端单元格").unwrap();
        let bytes = workbook.save_to_buffer().unwrap();
        assert!(bytes.len() < 20_000);
        let error = parse("稀疏范围.xlsx", &bytes).unwrap_err();
        assert!(error.1.contains("10000 行限制"));
    }

    #[test]
    fn sparse_xlsx_keeps_original_row_numbers_and_column_alignment() {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_name("每日训练记录").unwrap();
        sheet.write_string(5, 2, "日期").unwrap();
        sheet.write_string(5, 3, "训练记录").unwrap();
        sheet.write_string(8, 2, "2026-10-17").unwrap();
        sheet.write_string(8, 3, "合成观察").unwrap();
        let bytes = workbook.save_to_buffer().unwrap();
        let parsed = parse("有空行.xlsx", &bytes).unwrap();
        assert_eq!(parsed.sheets[0].headers, ["日期", "训练记录"]);
        assert_eq!(parsed.sheets[0].rows.len(), 1);
        assert_eq!(parsed.sheets[0].rows[0].row, 9);
        assert_eq!(parsed.sheets[0].rows[0].cells, ["2026-10-17", "合成观察"]);
    }

    #[test]
    fn excel_dates_preserve_1900_and_1904_epochs() {
        use calamine::{ExcelDateTime, ExcelDateTimeType};
        let legacy = DataRef::DateTime(ExcelDateTime::new(
            1462.5,
            ExcelDateTimeType::DateTime,
            false,
        ));
        let mac = DataRef::DateTime(ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, true));
        assert_eq!(cell_text(&legacy), "1904-01-01T12:00:00.000");
        assert_eq!(cell_text(&mac), "1904-01-01T12:00:00.000");
    }
}
