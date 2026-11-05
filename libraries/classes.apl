# Runtime storage for APL identity-bearing classes.
# Class syntax is lowered by the APL frontend to these table operations.

AVStr classes._magic = "APL_CLASS_1"
AVInt classes._next_id = 1
List classes._objects = []

func classes._handle_id(value) {
  if std::len(value) != 2 {
    return NONE
  }
  if std::get(value, 0) != classes._magic {
    return NONE
  }
  return std::int(std::get(value, 1))
}

func classes._record_index(value) {
  VTime requested = classes._handle_id(value)
  if requested == NONE {
    return NONE
  }

  VTime index = 0
  while (index < std::len(classes._objects)) (-1) {
    VTime record = std::get(classes._objects, index)
    if std::get(record, 0) == requested {
      return index
    }
    index += 1
  }
  return NONE
}

func classes._field_index(record, field) {
  VTime names = std::get(record, 3)
  VTime index = 0
  while (index < std::len(names)) (-1) {
    if std::get(names, index) == field {
      return index
    }
    index += 1
  }
  return NONE
}

func classes._new(type_name, ancestry, field_names, field_values) {
  if std::len(field_names) != std::len(field_values) {
    return NONE
  }

  VTime id = classes._next_id
  classes._next_id += 1
  std::add(classes._objects, [id, type_name, ancestry, field_names, field_values])
  return [classes._magic, id]
}

func classes._get(value, field) {
  VTime record_index = classes._record_index(value)
  if record_index == NONE {
    return NONE
  }
  VTime record = std::get(classes._objects, record_index)
  VTime field_index = classes._field_index(record, field)
  if field_index == NONE {
    return NONE
  }
  return std::get(std::get(record, 4), field_index)
}

func classes._set(value, field, replacement) {
  VTime record_index = classes._record_index(value)
  if record_index == NONE {
    return NONE
  }
  VTime record = std::get(classes._objects, record_index)
  VTime field_index = classes._field_index(record, field)
  if field_index == NONE {
    return NONE
  }
  VTime values = std::get(record, 4)
  std::set(values, field_index, replacement)
  std::set(record, 4, values)
  std::set(classes._objects, record_index, record)
  return replacement
}

func classes.id(value) {
  return classes._handle_id(value)
}

func classes.type(value) {
  VTime record_index = classes._record_index(value)
  if record_index == NONE {
    return NONE
  }
  return std::get(std::get(classes._objects, record_index), 1)
}

func classes.is_a(value, type_name) {
  VTime record_index = classes._record_index(value)
  if record_index == NONE {
    return false
  }
  VTime ancestry = std::get(std::get(classes._objects, record_index), 2)
  pick(ancestry): ancestor {
    if ancestor == type_name {
      return true
    }
  }
  return false
}
