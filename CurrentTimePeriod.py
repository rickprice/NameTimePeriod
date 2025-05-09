#!/usr/bin/env python3

import yaml
from datetime import datetime, timedelta

def load_yaml(filename):
    with open(filename, 'r') as file:
        return yaml.safe_load(file)

def is_within_period(current_date, base_date, days_before, days_after):
    start_date = base_date - timedelta(days=days_before)
    end_date = base_date + timedelta(days=days_after)
    return start_date <= current_date <= end_date

def get_current_period(yaml_data):
    current_date = datetime.now().date()
    current_year = current_date.year

    for item in yaml_data['TimePeriods']:
        for name, values in item.items():
            try:
                # Parse date using current year
                base_date = datetime.strptime(f"{values['Date']} {current_year}", "%B %d %Y").date()
                days_before = int(values.get('DaysBefore', 0))
                days_after = int(values.get('DaysAfter', 0))

                if is_within_period(current_date, base_date, days_before, days_after):
                    return name  # Return first matching period
            except ValueError as e:
                continue  # Skip invalid date entries

    return "Default"

if __name__ == "__main__":
    yaml_file = 'time_periods.yaml'  # Update if your file has a different name
    yaml_data = load_yaml(yaml_file)
    print(get_current_period(yaml_data))
