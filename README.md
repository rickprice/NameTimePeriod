# NameTimePeriod

A simple and extensible command-line tool written in Rust to determine which named time period (like "Mother's Day" or "Easter") a given date falls into, based on configurable YAML definitions.

## ✨ Features

- ✅ Supports flexible date definitions like:
  - `The second Sunday of May`
  - `Easter`, `Thanksgiving`, `LaborDay`, `MLKDay`, etc.
- ✅ Configurable `days_before` and `days_after` buffer windows.
- ✅ System and user configuration file support with merging.
- ✅ Command-line override of the date to test.
- ✅ Generates a default config file if none exists.

## 📦 Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (version 1.70+ recommended)

### Clone and build:

```bash
git clone https://github.com/yourusername/NameTimePeriod.git
cd NameTimePeriod
cargo build --release
```

### Or run with `rust-script`:

```bash
cargo install rust-script
rust-script src/main.rs
```

## 🚀 Usage

```bash
name_time_period              # Checks today's date
name_time_period --date 2025-05-11
name_time_period --init      # Force (re)create user config
```

## 🔧 Configuration

### System Config

- `/etc/NameTimePeriod/time_periods.yaml` (global, optional)

### User Config

- `~/.config/NameTimePeriod/time_periods.yaml`
- Created automatically on first run or with `--init`

### Example `time_periods.yaml`

```yaml
TimePeriods:
  - MothersDay:
      Date: The second Sunday of May
      DaysBefore: 3
      DaysAfter: 1
      Comment: Mother's Day
  - EasterPeriod:
      Date: Easter
      DaysBefore: 5
      DaysAfter: 2
```

> Entries are evaluated **in order**, and the first match wins.

## 🧪 Running Tests

```bash
cargo test
```

## 📁 Directory Structure

```
NameTimePeriod/
├── src/
│   └── main.rs         # Main logic
├── Cargo.toml
└── README.md
```

## 🙋 FAQ

**Q: What happens if the same key appears in both system and user configs?**  
A: The user config takes precedence and overrides the system config for that entry.

**Q: Can I define custom holidays?**  
A: Yes! Just add them to the YAML using a flexible date or standard date format.

## 📜 License

MIT License. See `LICENSE` for details.

## ✍️ Author

Frederick Price
