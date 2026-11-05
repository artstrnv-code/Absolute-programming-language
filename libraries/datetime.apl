AVInt datetime.TIMEZONE_OFFSET_MINUTES = 0

func datetime.now_utc() {
  return std::now_utc()
}

func datetime.now_local() {
  VTime __datetime_utc = datetime.now_utc()
  VTime __datetime_offset = datetime.TIMEZONE_OFFSET_MINUTES
  if (__datetime_offset < -1440) or (__datetime_offset > 1440) {
    return NONE
  }

  VTime __datetime_seconds = get(__datetime_utc, 3) * 3600 + get(__datetime_utc, 4) * 60 + get(__datetime_utc, 5) + __datetime_offset * 60
  VTime __datetime_day_offset = 0
  if __datetime_seconds < 0 {
    __datetime_seconds += 86400
    __datetime_day_offset = -1
  }
  if __datetime_seconds >= 86400 {
    __datetime_seconds -= 86400
    __datetime_day_offset = 1
  }
  VTime __datetime_date = datetime.add_days(get(__datetime_utc, 0), get(__datetime_utc, 1), get(__datetime_utc, 2), __datetime_day_offset)
  if __datetime_date == NONE {
    return NONE
  }
  return [
    get(__datetime_date, 0),
    get(__datetime_date, 1),
    get(__datetime_date, 2),
    __datetime_seconds / 3600,
    (__datetime_seconds / 60) % 60,
    __datetime_seconds % 60,
    get(__datetime_utc, 6)
  ]
}

func datetime._days_before_year(year) {
  VTime __datetime_years = year - 1
  return 365 * __datetime_years + __datetime_years / 4 - __datetime_years / 100 + __datetime_years / 400
}

func datetime._max_day_number() {
  return datetime._days_before_year(1000000000001) - 1
}

func datetime.is_leap_year(year) {
  VTime __datetime_year = std::int(year)
  if __datetime_year == NONE {
    return NONE
  }
  if (__datetime_year < 1) or (__datetime_year > 1000000000000) {
    return NONE
  }

  return ((__datetime_year % 4 == 0) and (__datetime_year % 100 != 0)) or (__datetime_year % 400 == 0)
}

func datetime.days_in_month(year, month) {
  VTime __datetime_year = std::int(year)
  VTime __datetime_month = std::int(month)
  if __datetime_year == NONE {
    return NONE
  }
  if __datetime_month == NONE {
    return NONE
  }
  if (datetime.is_leap_year(__datetime_year) == NONE) or (__datetime_month < 1) or (__datetime_month > 12) {
    return NONE
  }

  if (__datetime_month == 4) or (__datetime_month == 6) or (__datetime_month == 9) or (__datetime_month == 11) {
    return 30
  }
  if __datetime_month == 2 {
    if datetime.is_leap_year(__datetime_year) {
      return 29
    }
    return 28
  }
  return 31
}

func datetime.is_valid_date(year, month, day) {
  VTime __datetime_year = std::int(year)
  VTime __datetime_month = std::int(month)
  VTime __datetime_day = std::int(day)
  if __datetime_year == NONE {
    return false
  }
  if __datetime_month == NONE {
    return false
  }
  if __datetime_day == NONE {
    return false
  }
  VTime __datetime_month_length = datetime.days_in_month(__datetime_year, __datetime_month)
  if __datetime_month_length == NONE {
    return false
  }
  if (__datetime_day < 1) or (__datetime_day > __datetime_month_length) {
    return false
  }
  return true
}

func datetime.to_day_number(year, month, day) {
  VTime __datetime_year = std::int(year)
  VTime __datetime_month = std::int(month)
  VTime __datetime_day = std::int(day)
  if datetime.is_valid_date(__datetime_year, __datetime_month, __datetime_day) == false {
    return NONE
  }

  VTime __datetime_result = datetime._days_before_year(__datetime_year)
  VTime __datetime_index = 1
  while (__datetime_index < __datetime_month)(-1) {
    __datetime_result += datetime.days_in_month(__datetime_year, __datetime_index)
    __datetime_index += 1
  }
  return __datetime_result + __datetime_day - 1
}

func datetime.from_day_number(day_number) {
  VTime __datetime_number = std::int(day_number)
  VTime __datetime_max_year = 1000000000000
  if __datetime_number == NONE {
    return NONE
  }
  if (__datetime_number < 0) or (__datetime_number > datetime._max_day_number()) {
    return NONE
  }

  VTime __datetime_low = 1
  VTime __datetime_high = __datetime_max_year
  while (__datetime_low < __datetime_high)(-1) {
    VTime __datetime_mid = (__datetime_low + __datetime_high + 1) / 2
    if datetime._days_before_year(__datetime_mid) <= __datetime_number {
      __datetime_low = __datetime_mid
    } else {
      __datetime_high = __datetime_mid - 1
    }
  }

  VTime __datetime_day_of_year = __datetime_number - datetime._days_before_year(__datetime_low)
  VTime __datetime_month = 1
  while (__datetime_day_of_year >= datetime.days_in_month(__datetime_low, __datetime_month))(-1) {
    __datetime_day_of_year -= datetime.days_in_month(__datetime_low, __datetime_month)
    __datetime_month += 1
  }
  return [__datetime_low, __datetime_month, __datetime_day_of_year + 1]
}

func datetime.weekday(year, month, day) {
  VTime __datetime_number = datetime.to_day_number(year, month, day)
  if __datetime_number == NONE {
    return NONE
  }
  return (__datetime_number % 7) + 1
}

func datetime.add_days(year, month, day, offset) {
  VTime __datetime_number = datetime.to_day_number(year, month, day)
  VTime __datetime_offset = std::int(offset)
  if __datetime_number == NONE {
    return NONE
  }
  if __datetime_offset == NONE {
    return NONE
  }

  VTime __datetime_max = datetime._max_day_number()
  if (__datetime_offset > __datetime_max - __datetime_number) or (__datetime_offset < (0 - __datetime_number)) {
    return NONE
  }
  return datetime.from_day_number(__datetime_number + __datetime_offset)
}

func datetime.days_between(year_a, month_a, day_a, year_b, month_b, day_b) {
  VTime __datetime_number_a = datetime.to_day_number(year_a, month_a, day_a)
  VTime __datetime_number_b = datetime.to_day_number(year_b, month_b, day_b)
  if __datetime_number_a == NONE {
    return NONE
  }
  if __datetime_number_b == NONE {
    return NONE
  }
  return __datetime_number_b - __datetime_number_a
}
