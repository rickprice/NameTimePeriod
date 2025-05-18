#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! chrono = { version = "0.4", features = ["serde"] }
//! clap = { version = "4.5", features = ["derive"] }
//! dirs = "5.0"
//! regex = "1.10"
//! serde = { version = "1.0", features = ["derive"] }
//! serde_yaml = "0.9"
//! ```

use clap::Parser;
use chrono::{Datelike, NaiveDate, Utc, Weekday};
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;
use regex::Regex;

/// CLI tool to determine if today's date falls within a configured time period.
#[derive(Parser)]
#[command(name = "TimePeriodChecker")]
#[command(author = "Your Name")]
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
  - MLKReflection:
      Date: MLKDay
      DaysBefore: 0
      DaysAfter: 1
  - TamaraBirthday:
      Date: February 6
      DaysBefore: 7
      DaysAfter: 0
  - EricBirthday:
      Date: August 29
      DaysBefore: 7
      DaysAfter: 0
  - FrederickBirthday:
      Date: December 31
      DaysBefore: 7
      DaysAfter: 0
"#;

fn main() {
    let cli = Cli::parse();

    if cli.init {
        let user_path = get_user_config_path().unwrap_or_default();
        write_user_config(&user_path, true);
        return;
    }

    let current_date = cli.date.unwrap_or_else(|| Utc::now().date_naive());

    let system_path = "/etc/NameTimePeriod/time_periods.yaml";
    let user_path = get_user_config_path().unwrap_or_default();

    write_user_config(&user_path, false);

    let mut merged = load_yaml_file(system_path);
    let user_data = load_yaml_file(&user_path);
    for (k, v) in user_data {
        merged.insert(k, v);
    }

    println!("{}", get_current_period(&merged, current_date));
}

fn get_user_config_path() -> Option<String> {
    dirs::home_dir().map(|p| p.join(".config/NameTimePeriod/time_periods.yaml").to_str()?.to_string())
}

fn write_user_config(path: &str, force: bool) {
    let config_path = Path::new(path);
    if config_path.exists() && !force {
        return;
    }

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = create_dir_all(parent) {
                eprintln!("Failed to create config directory {}: {}", parent.display(), e);
                return;
            }
        }
    }

    match write(config_path, DEFAULT_CONFIG_YAML) {
        Ok(_) => println!(
            "Default user config {}written to {}",
            if force { "(force) " } else { "" },
            path
        ),
        Err(e) => eprintln!("Failed to write user config: {}", e),
    }
}

fn load_yaml_file(path: &str) -> HashMap<String, TimePeriod> {
    let mut periods = HashMap::new();
    if let Ok(content) = read_to_string(path) {
        if let Ok(doc) = serde_yaml::from_str::<HashMap<String, Vec<HashMap<String, TimePeriod>>>>(&content) {
            if let Some(tp) = doc.get("TimePeriods") {
                for item in tp {
                    for (name, value) in item {
                        periods.insert(name.clone(), value.clone());
                    }
                }
            }
        }
    }
    periods
}

#[derive(Debug, Clone, Deserialize)]
struct TimePeriod {
    Date: String,
    DaysBefore: i64,
    DaysAfter: i64,
    #[serde(default)]
    Comment: Option<String>,
}

fn get_current_period(periods: &HashMap<String, TimePeriod>, current_date: NaiveDate) -> String {
    for (name, period) in periods {
        if let Some(base_date) = parse_flexible_date(&period.Date, current_date.year()) {
            let start = base_date - chrono::Duration::days(period.DaysBefore);
            let end = base_date + chrono::Duration::days(period.DaysAfter);
            if current_date >= start && current_date <= end {
                return name.clone();
            }
        }
    }
    "Default".to_string()
}

fn parse_flexible_date(date_str: &str, year: i32) -> Option<NaiveDate> {
    let lower = date_str.trim().to_lowercase();

    match lower.as_str() {
        "easter" => return Some(calculate_easter(year)),
        "thanksgiving" => return nth_weekday_of_month(year, 11, Weekday::Thu, 4),
        "laborday" => return nth_weekday_of_month(year, 9, Weekday::Mon, 1),
        "memorialday" => return last_weekday_of_month(year, 5, Weekday::Mon),
        "mlkday" => return nth_weekday_of_month(year, 1, Weekday::Mon, 3),
        _ => {}
    }

    let re = Regex::new(r"(?i)the\s+(\w+)\s+(\w+)\s+of\s+(\w+)").unwrap();
    if let Some(cap) = re.captures(date_str) {
        let ordinal = &cap[1];
        let weekday_str = &cap[2];
        let month_str = &cap[3];

        let nth = match ordinal.to_lowercase().as_str() {
            "first" => 1,
            "second" => 2,
            "third" => 3,
            "fourth" => 4,
            "fifth" => 5,
            _ => return None,
        };

        let weekday = match weekday_str.to_lowercase().as_str() {
            "monday" => Weekday::Mon,
            "tuesday" => Weekday::Tue,
            "wednesday" => Weekday::Wed,
            "thursday" => Weekday::Thu,
            "friday" => Weekday::Fri,
            "saturday" => Weekday::Sat,
            "sunday" => Weekday::Sun,
            _ => return None,
        };

        let month = match chrono::Month::from_str(month_str) {
            Ok(m) => m.number_from_month(),
            Err(_) => return None,
        };

        return nth_weekday_of_month(year, month, weekday, nth);
    }

    NaiveDate::parse_from_str(&format!("{} {}", date_str, year), "%B %d %Y").ok()
}

fn nth_weekday_of_month(year: i32, month: u32, weekday: Weekday, nth: i64) -> Option<NaiveDate> {
    let mut count = 0;
    for day in 1..=31 {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            if date.weekday() == weekday {
                count += 1;
                if count == nth {
                    return Some(date);
                }
            }
        }
    }
    None
}

fn last_weekday_of_month(year: i32, month: u32, weekday: Weekday) -> Option<NaiveDate> {
    for day in (1..=31).rev() {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            if date.month() == month && date.weekday() == weekday {
                return Some(date);
            }
        }
    }
    None
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
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

