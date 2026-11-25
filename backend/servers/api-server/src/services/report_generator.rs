//! Report generator (gap audit 3.1: reports must actually produce files).
//!
//! Builds the four built-in reports (fault statistics, voting participation,
//! occupancy, consumption) from live data into a format-neutral
//! [`ReportDocument`], then renders it as CSV, XLSX or PDF. Used by:
//!
//! - `POST /api/v1/reports/export` for small reports (inline response),
//! - the [`report_worker`](super::report_worker) for large exports
//!   (`background_jobs` queue `reports`) and for scheduled report executions.
//!
//! There is deliberately no report-definitions table (#2198): a schedule's
//! `report_id` is one of the well-known built-in ids from ppt-web's
//! `builtinReports.ts`, mapped here by [`ReportKind::from_builtin_id`].

use chrono::NaiveDate;
use db::repositories::{FaultRepository, MeterRepository, PersonMonthRepository, VoteRepository};
use db::DbPool;
use uuid::Uuid;

use crate::routes::admin::audit::sanitize_csv_cell;

/// Default look-back window when a request gives no `from_date`.
pub const DEFAULT_REPORT_DAYS: i64 = 30;

/// The built-in report types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Faults,
    Voting,
    Occupancy,
    Consumption,
}

impl ReportKind {
    /// Parse the API `report_type` string.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "faults" => Some(Self::Faults),
            "voting" => Some(Self::Voting),
            "occupancy" => Some(Self::Occupancy),
            "consumption" => Some(Self::Consumption),
            _ => None,
        }
    }

    /// Map a schedule's `report_id` (a well-known built-in id, see
    /// `frontend/apps/ppt-web/src/features/reports/builtinReports.ts`).
    pub fn from_builtin_id(id: Uuid) -> Option<Self> {
        match id.to_string().as_str() {
            "a0000000-0000-4000-8000-000000000001" => Some(Self::Faults),
            "a0000000-0000-4000-8000-000000000002" => Some(Self::Voting),
            "a0000000-0000-4000-8000-000000000003" => Some(Self::Occupancy),
            "a0000000-0000-4000-8000-000000000004" => Some(Self::Consumption),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Faults => "faults",
            Self::Voting => "voting",
            Self::Occupancy => "occupancy",
            Self::Consumption => "consumption",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Faults => "Fault statistics",
            Self::Voting => "Voting participation",
            Self::Occupancy => "Occupancy",
            Self::Consumption => "Consumption",
        }
    }
}

/// Output file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Csv,
    Excel,
    Pdf,
}

impl ReportFormat {
    /// Parse the API `format` string (`csv`, `excel`/`xlsx`, `pdf`).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "excel" | "xlsx" => Some(Self::Excel),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Excel => "excel",
            Self::Pdf => "pdf",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Excel => "xlsx",
            Self::Pdf => "pdf",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv",
            Self::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Pdf => "application/pdf",
        }
    }
}

/// What to generate.
#[derive(Debug, Clone)]
pub struct ReportParams {
    pub kind: ReportKind,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl ReportParams {
    /// Build params, defaulting the range to the last [`DEFAULT_REPORT_DAYS`].
    pub fn new(
        kind: ReportKind,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Self {
        let to = to.unwrap_or_else(|| chrono::Utc::now().date_naive());
        let from = from.unwrap_or_else(|| to - chrono::Duration::days(DEFAULT_REPORT_DAYS));
        Self {
            kind,
            organization_id,
            building_id,
            from,
            to,
        }
    }
}

/// One table of a report.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportTable {
    pub heading: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl ReportTable {
    fn new(heading: &str, columns: &[&str]) -> Self {
        Self {
            heading: heading.to_string(),
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }
}

/// A format-neutral report: title, period line and tables.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportDocument {
    pub title: String,
    pub subtitle: String,
    pub tables: Vec<ReportTable>,
}

impl ReportDocument {
    /// Total number of data rows (used to decide sync vs async export).
    pub fn row_count(&self) -> usize {
        self.tables.iter().map(|t| t.rows.len()).sum()
    }
}

/// A rendered report file.
#[derive(Debug, Clone)]
pub struct GeneratedReport {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub content_type: &'static str,
    pub format: ReportFormat,
}

/// Builds report documents from the database and renders them.
#[derive(Clone)]
pub struct ReportGenerator {
    faults: FaultRepository,
    votes: VoteRepository,
    person_months: PersonMonthRepository,
    meters: MeterRepository,
}

fn num(value: impl std::fmt::Display) -> String {
    value.to_string()
}

fn opt_f64(value: Option<f64>) -> String {
    value.map_or_else(|| "N/A".to_string(), |v| format!("{:.1}", v))
}

impl ReportGenerator {
    pub fn from_pool(pool: DbPool) -> Self {
        Self {
            faults: FaultRepository::new(pool.clone()),
            votes: VoteRepository::new(pool.clone()),
            person_months: PersonMonthRepository::new(pool.clone()),
            meters: MeterRepository::new(pool),
        }
    }

    /// Load the report data and lay it out as tables.
    pub async fn build_document(&self, params: &ReportParams) -> Result<ReportDocument, String> {
        let org = params.organization_id;
        let building = params.building_id;
        let (from, to) = (params.from, params.to);
        let mut tables = Vec::new();

        match params.kind {
            ReportKind::Faults => {
                let stats = self
                    .faults
                    .get_statistics(org, building)
                    .await
                    .map_err(|e| format!("Failed to load fault statistics: {}", e))?;
                let mut summary = ReportTable::new("Summary", &["Metric", "Value"]);
                summary.push(vec!["Total faults".into(), num(stats.total_count)]);
                summary.push(vec!["Open".into(), num(stats.open_count)]);
                summary.push(vec!["Closed".into(), num(stats.closed_count)]);
                summary.push(vec![
                    "Average resolution (hours)".into(),
                    opt_f64(stats.average_resolution_time_hours),
                ]);
                summary.push(vec!["Average rating".into(), opt_f64(stats.average_rating)]);
                tables.push(summary);

                let mut by_status = ReportTable::new("By status", &["Status", "Count"]);
                for s in &stats.by_status {
                    by_status.push(vec![s.status.clone(), num(s.count)]);
                }
                tables.push(by_status);

                let mut by_category = ReportTable::new("By category", &["Category", "Count"]);
                for c in &stats.by_category {
                    by_category.push(vec![c.category.clone(), num(c.count)]);
                }
                tables.push(by_category);

                let mut by_priority = ReportTable::new("By priority", &["Priority", "Count"]);
                for p in &stats.by_priority {
                    by_priority.push(vec![p.priority.clone(), num(p.count)]);
                }
                tables.push(by_priority);
            }
            ReportKind::Voting => {
                let votes = self
                    .votes
                    .get_participation_report(org, building, from, to)
                    .await
                    .map_err(|e| format!("Failed to load voting participation: {}", e))?;
                let mut table = ReportTable::new(
                    "Votes",
                    &[
                        "Title",
                        "Status",
                        "Start",
                        "End",
                        "Eligible",
                        "Responses",
                        "Participation",
                        "Quorum required",
                        "Quorum reached",
                    ],
                );
                for v in &votes {
                    table.push(vec![
                        v.title.clone(),
                        v.status.clone(),
                        v.start_at.clone().unwrap_or_else(|| "N/A".into()),
                        v.end_at.clone(),
                        num(v.eligible_count),
                        num(v.response_count),
                        format!("{:.1}%", v.participation_rate),
                        v.quorum_required
                            .map_or_else(|| "N/A".into(), |q| format!("{}%", q)),
                        if v.quorum_reached { "Yes" } else { "No" }.into(),
                    ]);
                }
                tables.push(table);
            }
            ReportKind::Occupancy => {
                let data = self
                    .person_months
                    .get_occupancy_report(org, building, from, to)
                    .await
                    .map_err(|e| format!("Failed to load occupancy: {}", e))?;
                let mut summary = ReportTable::new("Summary", &["Metric", "Value"]);
                summary.push(vec!["Total units".into(), num(data.summary.total_units)]);
                summary.push(vec![
                    "Occupied units".into(),
                    num(data.summary.occupied_units),
                ]);
                summary.push(vec!["Vacant units".into(), num(data.summary.vacant_units)]);
                summary.push(vec![
                    "Occupancy rate".into(),
                    format!("{:.1}%", data.summary.occupancy_rate),
                ]);
                summary.push(vec![
                    "Total person-months".into(),
                    num(data.summary.total_person_months),
                ]);
                tables.push(summary);

                let mut by_unit =
                    ReportTable::new("By unit", &["Unit", "Person-months", "Average occupants"]);
                for u in &data.by_unit {
                    by_unit.push(vec![
                        u.unit_designation.clone(),
                        num(u.total_person_months),
                        format!("{:.2}", u.average_occupants),
                    ]);
                }
                tables.push(by_unit);

                let mut monthly = ReportTable::new("By month", &["Month", "Person-months"]);
                for m in &data.monthly_totals {
                    monthly.push(vec![format!("{}-{:02}", m.year, m.month), num(m.count)]);
                }
                tables.push(monthly);
            }
            ReportKind::Consumption => {
                let data = self
                    .meters
                    .get_consumption_report(org, building, None, from, to)
                    .await
                    .map_err(|e| format!("Failed to load consumption: {}", e))?;
                let mut summary = ReportTable::new("Summary", &["Metric", "Value"]);
                summary.push(vec![
                    "Total consumption".into(),
                    num(data.summary.total_consumption),
                ]);
                summary.push(vec!["Total cost".into(), num(data.summary.total_cost)]);
                summary.push(vec!["Meters".into(), num(data.summary.meter_count)]);
                summary.push(vec![
                    "Average consumption per unit".into(),
                    num(data.summary.average_consumption_per_unit),
                ]);
                tables.push(summary);

                let mut by_type = ReportTable::new(
                    "By utility type",
                    &["Utility", "Consumption", "Unit", "Cost", "Meters"],
                );
                for t in &data.by_utility_type {
                    by_type.push(vec![
                        t.utility_type.clone(),
                        num(t.total_consumption),
                        t.unit.clone(),
                        num(t.total_cost),
                        num(t.meter_count),
                    ]);
                }
                tables.push(by_type);

                let mut by_unit = ReportTable::new(
                    "By unit",
                    &[
                        "Unit",
                        "Utility",
                        "Consumption",
                        "Monthly average",
                        "Deviation",
                    ],
                );
                for u in &data.by_unit {
                    by_unit.push(vec![
                        u.unit_designation.clone(),
                        u.utility_type.clone(),
                        num(u.total_consumption),
                        num(u.average_monthly),
                        format!("{:+.1}%", u.deviation_percentage),
                    ]);
                }
                tables.push(by_unit);
            }
        }

        Ok(ReportDocument {
            title: params.kind.title().to_string(),
            subtitle: format!("Period: {} to {}", from, to),
            tables,
        })
    }

    /// Build and render a report.
    pub async fn generate(
        &self,
        params: &ReportParams,
        format: ReportFormat,
    ) -> Result<GeneratedReport, String> {
        let document = self.build_document(params).await?;
        render(&document, params.kind, format)
    }
}

/// Render a document in the requested format.
pub fn render(
    document: &ReportDocument,
    kind: ReportKind,
    format: ReportFormat,
) -> Result<GeneratedReport, String> {
    let bytes = match format {
        ReportFormat::Csv => render_csv(document)?,
        ReportFormat::Excel => render_xlsx(document)?,
        ReportFormat::Pdf => render_pdf(document)?,
    };
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    Ok(GeneratedReport {
        bytes,
        file_name: format!("{}_{}.{}", kind.as_str(), timestamp, format.extension()),
        content_type: format.content_type(),
        format,
    })
}

/// Neutralise a user-authored cell for spreadsheet output (formula injection,
/// embedded CR/LF — see `sanitize_csv_cell`).
fn safe_cell(value: &str) -> String {
    sanitize_csv_cell(value)
}

/// CSV: one block per table (heading row, column row, data rows) separated by
/// a blank line. Quoting is done by the `csv` crate.
pub fn render_csv(document: &ReportDocument) -> Result<Vec<u8>, String> {
    let mut writer = csv::WriterBuilder::new()
        .flexible(true)
        .from_writer(Vec::new());
    let write_err = |e: csv::Error| format!("CSV write error: {}", e);

    writer
        .write_record([document.title.as_str()])
        .map_err(write_err)?;
    writer
        .write_record([document.subtitle.as_str()])
        .map_err(write_err)?;
    for table in &document.tables {
        writer.write_record([""]).map_err(write_err)?;
        writer
            .write_record([table.heading.as_str()])
            .map_err(write_err)?;
        writer.write_record(&table.columns).map_err(write_err)?;
        for row in &table.rows {
            writer
                .write_record(row.iter().map(|c| safe_cell(c)))
                .map_err(write_err)?;
        }
    }
    writer
        .into_inner()
        .map_err(|e| format!("CSV flush error: {}", e))
}

/// Worksheet names: max 31 chars, no `[]:*?/\`.
fn sheet_name(heading: &str, index: usize) -> String {
    let cleaned: String = heading
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .take(28)
        .collect();
    if cleaned.trim().is_empty() {
        format!("Sheet{}", index + 1)
    } else {
        cleaned
    }
}

/// XLSX: one worksheet per table. Numeric-looking cells are written as numbers.
pub fn render_xlsx(document: &ReportDocument) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Format, Workbook};

    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let err = |e: rust_xlsxwriter::XlsxError| e.to_string();

    for (index, table) in document.tables.iter().enumerate() {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name(sheet_name(&table.heading, index))
            .map_err(err)?;
        sheet
            .write_string_with_format(0, 0, &document.title, &bold)
            .map_err(err)?;
        sheet.write_string(1, 0, &document.subtitle).map_err(err)?;
        for (col, name) in table.columns.iter().enumerate() {
            sheet
                .write_string_with_format(3, col as u16, name, &bold)
                .map_err(err)?;
            sheet.set_column_width(col as u16, 18).map_err(err)?;
        }
        for (r, row) in table.rows.iter().enumerate() {
            let row_idx = 4 + r as u32;
            for (col, value) in row.iter().enumerate() {
                match value.parse::<f64>() {
                    Ok(n) if n.is_finite() => {
                        sheet.write_number(row_idx, col as u16, n).map_err(err)?;
                    }
                    _ => {
                        sheet
                            .write_string(row_idx, col as u16, safe_cell(value))
                            .map_err(err)?;
                    }
                }
            }
        }
    }
    if document.tables.is_empty() {
        let sheet = workbook.add_worksheet();
        sheet.write_string(0, 0, &document.title).map_err(err)?;
    }
    workbook.save_to_buffer().map_err(err)
}

/// Directory of the TrueType fonts used for PDFs (same as the vote and
/// financial PDF exports; installed via `fonts-liberation` in the image).
const PDF_FONT_DIR: &str = "/usr/share/fonts/truetype/liberation";

/// PDF: title, period, then each table with a header row.
pub fn render_pdf(document: &ReportDocument) -> Result<Vec<u8>, String> {
    use genpdf::{elements, style::Style, Element};

    let fonts = genpdf::fonts::from_files(PDF_FONT_DIR, "LiberationSans", None)
        .map_err(|e| format!("Failed to load font LiberationSans: {}", e))?;
    let mut doc = genpdf::Document::new(fonts);
    doc.set_title(document.title.clone());
    doc.set_font_size(10);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new(document.title.clone())
            .styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Paragraph::new(document.subtitle.clone()));

    for table in &document.tables {
        doc.push(elements::Break::new(1));
        doc.push(
            elements::Paragraph::new(table.heading.clone())
                .styled(Style::new().bold().with_font_size(13)),
        );
        if table.columns.is_empty() {
            continue;
        }
        let mut layout = elements::TableLayout::new(vec![1; table.columns.len()]);
        layout.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
        let mut header = layout.row();
        for column in &table.columns {
            header.push_element(
                elements::Paragraph::new(column.clone())
                    .styled(Style::new().bold())
                    .padded(1),
            );
        }
        header
            .push()
            .map_err(|e| format!("PDF table error: {}", e))?;
        if table.rows.is_empty() {
            let mut row = layout.row();
            for (i, _) in table.columns.iter().enumerate() {
                let text = if i == 0 { "No data" } else { "" };
                row.push_element(elements::Paragraph::new(text).padded(1));
            }
            row.push().map_err(|e| format!("PDF table error: {}", e))?;
        }
        for data in &table.rows {
            let mut row = layout.row();
            for i in 0..table.columns.len() {
                let value = data.get(i).cloned().unwrap_or_default();
                row.push_element(elements::Paragraph::new(value).padded(1));
            }
            row.push().map_err(|e| format!("PDF table error: {}", e))?;
        }
        doc.push(layout);
    }

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("PDF render error: {}", e))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ReportDocument {
        let mut table = ReportTable::new("Votes", &["Title", "Responses"]);
        table.push(vec!["=cmd|'/c calc'!A1".into(), "8".into()]);
        table.push(vec!["Roof\r\nInjected,row".into(), "3".into()]);
        ReportDocument {
            title: "Voting participation".into(),
            subtitle: "Period: 2026-01-01 to 2026-12-31".into(),
            tables: vec![table, ReportTable::new("Empty", &["A"])],
        }
    }

    #[test]
    fn kinds_parse_and_map_builtin_ids() {
        assert_eq!(ReportKind::parse("Faults"), Some(ReportKind::Faults));
        assert_eq!(ReportKind::parse("nope"), None);
        let id = Uuid::parse_str("a0000000-0000-4000-8000-000000000004").unwrap();
        assert_eq!(
            ReportKind::from_builtin_id(id),
            Some(ReportKind::Consumption)
        );
        assert_eq!(ReportKind::from_builtin_id(Uuid::new_v4()), None);
    }

    #[test]
    fn formats_parse_with_extensions() {
        assert_eq!(ReportFormat::parse("xlsx"), Some(ReportFormat::Excel));
        assert_eq!(ReportFormat::parse("excel").unwrap().extension(), "xlsx");
        assert_eq!(ReportFormat::parse("PDF"), Some(ReportFormat::Pdf));
        assert_eq!(ReportFormat::parse("doc"), None);
    }

    #[test]
    fn csv_neutralises_formulas_and_line_breaks() {
        let bytes = render_csv(&sample()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(text.as_bytes());
        let records: Vec<csv::StringRecord> = reader.records().map(|r| r.unwrap()).collect();
        let formula_row = records
            .iter()
            .find(|r| r.get(1) == Some("8"))
            .expect("data row");
        assert!(formula_row.get(0).unwrap().starts_with('\''));
        let crlf_row = records
            .iter()
            .find(|r| r.get(1) == Some("3"))
            .expect("data row");
        assert!(!crlf_row.get(0).unwrap().contains('\n'));
        // Comma stays inside the quoted cell instead of spilling columns.
        assert_eq!(crlf_row.len(), 2);
    }

    #[test]
    fn xlsx_renders_a_zip_container() {
        let bytes = render_xlsx(&sample()).unwrap();
        assert_eq!(&bytes[..2], b"PK");
    }

    #[test]
    fn pdf_renders_when_fonts_are_installed() {
        if !std::path::Path::new(PDF_FONT_DIR)
            .join("LiberationSans-Regular.ttf")
            .exists()
        {
            eprintln!("skipping: Liberation fonts not installed");
            return;
        }
        let bytes = render_pdf(&sample()).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn sheet_names_are_valid() {
        assert_eq!(sheet_name("By status", 0), "By status");
        assert_eq!(sheet_name("a/b:c", 0), "abc");
        assert_eq!(sheet_name("???", 2), "Sheet3");
        assert!(sheet_name(&"x".repeat(60), 0).len() <= 31);
    }

    #[test]
    fn row_count_sums_tables() {
        assert_eq!(sample().row_count(), 2);
    }
}
