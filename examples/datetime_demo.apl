include >> datetime
exclude "_days_before_year" << datetime
exclude "_max_day_number" << datetime

AVInt std::main() {
  out datetime::is_leap_year(2000)
  out "\n"
  out datetime::is_leap_year(1900)
  out "\n"
  out datetime::days_in_month(2024, 2)
  out "\n"
  out datetime::is_valid_date(2023, 2, 29)
  out "\n"
  out datetime::is_valid_date(0, 1, 1)
  out "\n"
  out datetime::is_valid_date(2024, 13, 1)
  out "\n"
  out datetime::to_day_number(1, 1, 1)
  out "\n"
  out datetime::weekday(1, 1, 1)
  out "\n"
  out datetime::weekday(1970, 1, 1)
  out "\n"
  out datetime::weekday(2000, 1, 1)
  out "\n"
  out datetime::from_day_number(datetime::to_day_number(2000, 2, 28))
  out "\n"
  out datetime::add_days(2000, 2, 28, 1)
  out "\n"
  out datetime::add_days(2000, 3, 1, -1)
  out "\n"
  out datetime::days_between(2024, 2, 28, 2024, 3, 1)
  out "\n"
  out datetime::add_days(1, 1, 1, -1)
  out "\n"
  out datetime::from_day_number(datetime::to_day_number(1000000000000, 12, 31))
  out "\n"
  out datetime::add_days(1000000000000, 12, 31, 1)
  out "\n"
  out datetime::is_leap_year(0)
  out "\nUTC [year, month, day, hour, minute, second, millisecond]: "
  out datetime::now_utc()
  out "\nFixed-offset local time: "
  out datetime::now_local()
  return 0
}
