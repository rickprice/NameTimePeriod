// #!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! chrono = { version = "0.4", features = ["serde"] }
//! clap = { version = "4.5", features = ["derive"] }
//! dirs = "5.0"
//! regex = "1.10"
//! serde = { version = "1.0", features = ["derive"] }
//! serde_yaml = "0.9"
//! ```

use chrono::{Datelike, NaiveDate, Utc, Weekday};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::fmt;
use std::fs::{create_dir_all, read_to_string, write};
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
///
/// CLI tool to determine if today's date falls within a configured time period.
#[derive(Parser)]
#[command(name = "TimePeriodChecker")]
#[command(author = "Frederick Price")]
#[command(version = "1.0")]
#[command(about = "Checks what time period a date falls into based on YAML configs", long_about = None)]
struct Cli {
    /// Pass a specific date to evaluate (format: YYYY-MM-DD)
    #[arg(short, long)]
    date: Option<NaiveDate>,

    /// Force-regenerate the user config file
    #[arg(long)]
    init: bool,
}

const DEFAULT_CONFIG_YAML: &str = r#"TimePeriods:
  - MothersDay:
      Date: The second Sunday of May
      DaysBefore: 3
      DaysAfter: 1
      Comment: Mother's Day
  - FathersDay:
      Date: The third Sunday of June
      DaysBefore: 3
      DaysAfter: 1
      Comment: Father's Day
  - EasterPeriod:
      Date: Easter
      DaysBefore: 5
      DaysAfter: 2
  - Thanksgiving:
      Date: Thanksgiving
      DaysBefore: 3
      DaysAfter: 2
  - LaborWeek:
      Date: LaborDay
      DaysBefore: 1
      DaysAfter: 2
"#;

fn main() {
    let cli = Cli::parse();

    if cli.init {
        if let Some(user_path) = get_user_config_path() {
            if let Err(e) = write_user_config(&user_path, true) {
                eprintln!("Error: {}", e);
            }
        } else {
            eprintln!("Error: Could not determine user config path");
        }
        return;
    }

    let current_date = cli.date.unwrap_or_else(|| Utc::now().date_naive());

    let system_path = "/etc/NameTimePeriod/time_periods.yaml";
    let user_path = get_user_config_path();

    // Only create user config if both system and user configs don't exist
    if let Some(ref path) = user_path {
        let system_path_buf = Path::new(system_path);
        if !system_path_buf.exists() && !path.exists() {
            if let Err(e) = write_user_config(path, false) {
                eprintln!("Warning: {}", e);
            }
        }
    }

    let merged: Vec<_> = user_path
        .as_ref()
        .map(|path| load_yaml_file(path))
        .unwrap_or_default()
        .into_iter()
        .chain(load_yaml_file(Path::new(system_path)))
        .collect();

    println!("{}", get_current_period(&merged, current_date));
}

fn get_user_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config/NameTimePeriod/time_periods.yaml"))
}

#[derive(Debug)]
enum ConfigError {
    IoError(io::Error),
    DirectoryCreation(io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::DirectoryCreation(e) => write!(f, "Failed to create directory: {}", e),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        ConfigError::IoError(error)
    }
}

fn write_user_config(path: &Path, force: bool) -> Result<(), ConfigError> {
    if path.exists() && !force {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            create_dir_all(parent).map_err(ConfigError::DirectoryCreation)?;
        }
    }

    write(path, DEFAULT_CONFIG_YAML)?;
    println!(
        "Default user config {}written to {}",
        if force { "(force) " } else { "" },
        path.display()
    );
    Ok(())
}

fn load_yaml_file(path: &Path) -> Vec<(String, TimePeriod)> {
    load_yaml_file_inner(path).unwrap_or_default()
}

fn load_yaml_file_inner(path: &Path) -> Option<Vec<(String, TimePeriod)>> {
    let content = read_to_string(path).ok()?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let arr = doc.get("TimePeriods")?.as_sequence()?;

    Some(
        arr.iter()
            .filter_map(|entry| {
                let map = entry.as_mapping()?;
                map.iter().find_map(|(k, v)| {
                    let name = k.as_str()?.to_string();
                    let tp = serde_yaml::from_value::<TimePeriod>(v.clone()).ok()?;
                    Some((name, tp))
                })
            })
            .collect(),
    )
}

#[derive(Debug, Clone, Deserialize)]
struct TimePeriod {
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "DaysBefore")]
    days_before: i64,
    #[serde(rename = "DaysAfter")]
    days_after: i64,
    // #[serde(rename = "Comment")]
    // comment: Option<String>,
}

fn get_current_period(periods: &[(String, TimePeriod)], current_date: NaiveDate) -> String {
    let matches: Vec<&str> = periods
        .iter()
        .filter_map(|(name, period)| {
            let base_date = parse_flexible_date(&period.date, current_date.year())?;
            let start = base_date - chrono::Duration::days(period.days_before);
            let end = base_date + chrono::Duration::days(period.days_after);
            (start..=end)
                .contains(&current_date)
                .then_some(name.as_str())
        })
        .collect();

    if matches.is_empty() {
        "Default".to_string()
    } else {
        matches.join(" ")
    }
}

fn parse_flexible_date(date_str: &str, year: i32) -> Option<NaiveDate> {
    let lower = date_str.trim().to_lowercase();

    match lower.as_str() {
        "easter" => Some(calculate_easter(year)),
        "thanksgiving" => nth_weekday_of_month(year, 11, Weekday::Thu, 4),
        "laborday" => nth_weekday_of_month(year, 9, Weekday::Mon, 1),
        "memorialday" => last_weekday_of_month(year, 5, Weekday::Mon),
        "mlkday" => nth_weekday_of_month(year, 1, Weekday::Mon, 3),
        _ => parse_relative_date(date_str, year).or_else(|| {
            NaiveDate::parse_from_str(&format!("{} {}", date_str, year), "%B %d %Y").ok()
        }),
    }
}

static RELATIVE_DATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)the\s+(\w+)\s+(\w+)\s+of\s+(\w+)").expect("Invalid regex pattern")
});

fn parse_relative_date(date_str: &str, year: i32) -> Option<NaiveDate> {
    let cap = RELATIVE_DATE_REGEX.captures(date_str)?;

    let nth = match cap[1].to_lowercase().as_str() {
        "first" => 1,
        "second" => 2,
        "third" => 3,
        "fourth" => 4,
        "fifth" => 5,
        _ => return None,
    };

    let weekday = match cap[2].to_lowercase().as_str() {
        "monday" => Weekday::Mon,
        "tuesday" => Weekday::Tue,
        "wednesday" => Weekday::Wed,
        "thursday" => Weekday::Thu,
        "friday" => Weekday::Fri,
        "saturday" => Weekday::Sat,
        "sunday" => Weekday::Sun,
        _ => return None,
    };

    let month = chrono::Month::from_str(&cap[3]).ok()?.number_from_month();
    nth_weekday_of_month(year, month, weekday, nth)
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: i64) -> Option<NaiveDate> {
    (1..=31)
        .filter_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .filter(|date| date.weekday() == weekday)
        .nth(nth.saturating_sub(1) as usize)
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> Option<NaiveDate> {
    (1..=31)
        .rev()
        .filter_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .find(|date| date.weekday() == weekday)
}

fn calculate_easter(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        .expect("Invalid Easter date calculation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Weekday};

    #[test]
    fn test_fixed_date_parsing() {
        let date = parse_flexible_date("February 6", 2025);
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2025, 2, 6).unwrap()));
    }

    #[test]
    fn test_easter_date() {
        let easter = calculate_easter(2025);
        assert_eq!(easter, NaiveDate::from_ymd_opt(2025, 4, 20).unwrap());
    }

    #[test]
    fn test_nth_weekday_of_month() {
        let date = nth_weekday_of_month(2025, 5, Weekday::Sun, 2);
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2025, 5, 11).unwrap())); // 2nd Sunday of May
    }

    #[test]
    fn test_last_weekday_of_month() {
        let date = last_weekday_of_month(2025, 5, Weekday::Mon);
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2025, 5, 26).unwrap())); // Memorial Day
    }

    #[test]
    fn test_parse_the_second_sunday() {
        let date = parse_flexible_date("The second Sunday of May", 2025);
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2025, 5, 11).unwrap()));
    }

    #[test]
    fn test_parse_thanksgiving() {
        let date = parse_flexible_date("Thanksgiving", 2025);
        assert_eq!(date, Some(NaiveDate::from_ymd_opt(2025, 11, 27).unwrap())); // 4th Thursday of Nov
    }

    #[test]
    fn test_get_current_period_match() {
        let mut periods = Vec::new();
        periods.push((
            "MyPeriod".to_string(),
            TimePeriod {
                date: "February 6".to_string(),
                days_before: 2,
                days_after: 2,
                // comment: Some("Test".to_string()),
            },
        ));
        let test_date = NaiveDate::from_ymd_opt(2025, 2, 5).unwrap();
        assert_eq!(get_current_period(&periods, test_date), "MyPeriod");
    }

    #[test]
    fn test_get_current_period_multiple_matches() {
        let mut periods = Vec::new();
        periods.push((
            "Period1".to_string(),
            TimePeriod {
                date: "February 6".to_string(),
                days_before: 2,
                days_after: 2,
            },
        ));
        periods.push((
            "Period2".to_string(),
            TimePeriod {
                date: "February 6".to_string(),
                days_before: 1,
                days_after: 1,
            },
        ));
        let test_date = NaiveDate::from_ymd_opt(2025, 2, 6).unwrap();
        assert_eq!(get_current_period(&periods, test_date), "Period1 Period2");
    }

    #[test]
    fn test_get_current_period_no_match() {
        let mut periods = Vec::new();
        periods.push((
            "MyPeriod".to_string(),
            TimePeriod {
                date: "February 6".to_string(),
                days_before: 2,
                days_after: 2,
                // comment: Some("Test".to_string()),
            },
        ));
        let test_date = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        assert_eq!(get_current_period(&periods, test_date), "Default");
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml = r#"
Date: Easter
DaysBefore: 3
DaysAfter: 2
Comment: Easter celebration
"#;
        let tp: TimePeriod = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tp.date, "Easter");
        assert_eq!(tp.days_before, 3);
        assert_eq!(tp.days_after, 2);
        // assert_eq!(tp.comment.as_deref(), Some("Easter celebration"));
    }

    #[test]
    fn test_parse_invalid_flexible_date() {
        assert_eq!(parse_flexible_date("Invalid date", 2025), None);
    }
}
