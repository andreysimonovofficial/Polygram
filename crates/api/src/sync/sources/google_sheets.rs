//! Источник «Google Таблицы» и парсеры его ответов.

use polygram_core::types::{MetricKind, MetricValue, ParseHint};
use serde_json::Value;
use std::process::Command;

use crate::sync::MetricSource;

/// Источник: Google Таблицы.
pub struct GoogleSheetsSource;

impl MetricSource for GoogleSheetsSource {
    fn id(&self) -> &str {
        "google-sheets"
    }
    fn name(&self) -> &str {
        "Google Таблицы"
    }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![
            MetricKind::Number,
            MetricKind::Date,
            MetricKind::Deadline,
            MetricKind::Text,
        ]
    }
    fn fetch(
        &self,
        cell: &str,
        config: Option<&Value>,
        kind: MetricKind,
        parse_hint: &ParseHint,
    ) -> Result<MetricValue, String> {
        let spreadsheet_id = match config
            .and_then(|c| c.get("url").and_then(|v| v.as_str()))
            .and_then(extract_spreadsheet_id)
        {
            Some(id) => id,
            None => {
                return Err(
                    "не задан URL Google Таблицы в конфиге коннектора google-sheets. Укажите URL таблицы в сайдбаре «Коннекторы» → Google Таблицы"
                        .to_owned(),
                );
            }
        };

        let gws_path = gws_binary_path();

        let params = serde_json::json!({
            "spreadsheetId": spreadsheet_id,
            "ranges": [cell],
            "fields": "sheets(data(rowData(values(userEnteredValue,formattedValue))))"
        });

        let output = Command::new("cmd")
            .args([
                "/c",
                &gws_path,
                "sheets", "spreadsheets", "get",
                "--params", &params.to_string(),
                "--format", "json"
            ])
            .output()
            .map_err(|e| format!("не удалось запустить gws.exe ({gws_path}): {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "gws.exe завершился с ошибкой: stdout={stdout} stderr={stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_brace = match stdout.find('{') {
            Some(p) => p,
            None => {
                return Err(format!(
                    "не найден JSON в ответе gws.exe. stdout={stdout}"
                ));
            }
        };
        let last_brace = match stdout.rfind('}') {
            Some(p) => p,
            None => {
                return Err(format!(
                    "не найден закрывающий скобку JSON в ответе gws.exe. stdout={stdout}"
                ));
            }
        };
        let json_str = &stdout[first_brace..=last_brace];

        let resp: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => return Err(format!("не удалось распарсить ответ gws: {e}")),
        };

        let value = match resp.get("userEnteredValue") {
            Some(v) => v.clone(),
            None => resp
                .get("sheets")
                .and_then(|s| s.as_array())
                .and_then(|arr| arr.first())
                .and_then(|s| s.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|d| d.first())
                .and_then(|d| d.get("rowData"))
                .and_then(|rd| rd.as_array())
                .and_then(|arr| arr.first())
                .and_then(|rd| rd.get("values"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.get("formattedValue"))
                .and_then(|fv| fv.as_str())
                .map(|s| Value::String(s.to_owned()))
                .ok_or_else(|| {
                    format!(
                        "ячейка {cell} в таблице {spreadsheet_id} пуста"
                    )
                })?,
        };

        parse_google_sheets_value(&value, kind, parse_hint, cell, &spreadsheet_id)
    }
    fn placeholder(&self) -> &str {
        "https://docs.google.com/spreadsheets/d/... или Sheet1!A1"
    }
}

/// Извлечь ID таблицы из URL Google Таблиц.
fn extract_spreadsheet_id(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split("/d/").collect();
    if parts.len() < 2 {
        return None;
    }
    let rest = parts[1];
    let id = rest.split('/').next()?.split('?').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_owned())
    }
}

/// Путь к бинарнику gws.exe.
fn gws_binary_path() -> String {
    if Command::new("gws.exe").arg("--help").output().is_ok() {
        return "gws.exe".to_owned();
    }
    r"C:\Users\andre\AppData\Roaming\npm\node_modules\@googleworkspace\cli\bin\gws.exe"
        .to_owned()
}

/// Распарсить сырое значение Google Sheets в MetricValue согласно типу показателя.
fn parse_google_sheets_value(
    raw: &Value,
    kind: MetricKind,
    parse_hint: &ParseHint,
    cell: &str,
    spreadsheet_id: &str,
) -> Result<MetricValue, String> {
    match kind {
        MetricKind::Number => {
            if let Some(n) = raw.get("numberValue").and_then(|v| v.as_f64()) {
                return Ok(MetricValue::Number(n));
            }
            if let Some(s) = raw.as_str().and_then(|s| s.parse::<f64>().ok()) {
                return Ok(MetricValue::Number(s));
            }
            Err(format!(
                "ячейка {cell} в таблице {spreadsheet_id} не содержит число"
            ))
        }
        MetricKind::Date | MetricKind::Deadline => {
            if let Some(fmt) = raw.get("formattedValue").and_then(|v| v.as_str()) {
                if let Some(dt) = parse_date_string(fmt, parse_hint) {
                    return match kind {
                        MetricKind::Deadline => Ok(MetricValue::Deadline(dt)),
                        _ => Ok(MetricValue::Date(dt)),
                    };
                }
            }
            if let Some(n) = raw.get("numberValue").and_then(|v| v.as_f64()) {
                if let Some(dt) = serial_date_to_datetime(n) {
                    return match kind {
                        MetricKind::Deadline => Ok(MetricValue::Deadline(dt)),
                        _ => Ok(MetricValue::Date(dt)),
                    };
                }
            }
            Err(format!(
                "ячейка {cell} в таблице {spreadsheet_id} не содержит дату"
            ))
        }
        MetricKind::Text => {
            if let Some(s) = raw.get("formattedValue").and_then(|v| v.as_str()) {
                Ok(MetricValue::Text(s.to_owned()))
            } else if let Some(s) = raw.as_str() {
                Ok(MetricValue::Text(s.to_owned()))
            } else {
                Err(format!(
                    "ячейка {cell} в таблице {spreadsheet_id} не содержит текст"
                ))
            }
        }
        _ => Err(format!(
            "Google Таблицы не поддерживают тип {:?}",
            kind
        )),
    }
}

/// Распарсить строку даты в DateTime<Utc> согласно parse_hint.
fn parse_date_string(s: &str, parse_hint: &ParseHint) -> Option<chrono::DateTime<chrono::Utc>> {
    match parse_hint {
        ParseHint::DateFormat(fmt)
            if fmt == "ISO 8601" || fmt.is_empty() =>
        {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }
        ParseHint::DateFormat(fmt) => {
            chrono::NaiveDate::parse_from_str(s, fmt)
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc())
        }
        ParseHint::None => {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc())
                .or_else(|| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                })
        }
        _ => None,
    }
}

/// Преобразовать серийную дату Google Sheets в DateTime<Utc>.
fn serial_date_to_datetime(serial: f64) -> Option<chrono::DateTime<chrono::Utc>> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial as i64;
    let frac = serial.fract();
    let date = epoch.checked_add_signed(chrono::Duration::days(days))?;
    let seconds = (frac * 86400.0).round() as i64;
    let dt = date.and_hms_opt(0, 0, 0)?;
    let dt = dt.checked_add_signed(chrono::Duration::seconds(seconds))?;
    Some(dt.and_utc())
}
