# Value-structure lowering. Struct instances are immutable tagged value records;
# field assignments rebuild and rebind the affected path, which gives ordinary
# assignment value semantics without adding a second runtime object system.

AVStr structlower.MAGIC = "APL_STRUCT_1"

func structlower.ok(value) {
  return [true, value]
}

func structlower.fail(message, node) {
  return [false, message, node]
}

func structlower.name(node) {
  return get(node, 1)
}

func structlower.fields(node) {
  return get(node, 2)
}

func structlower.find(structs, name) {
  pick(structs): node {
    if structlower.name(node) == name {
      return node
    }
  }
  return NONE
}

func structlower.includes(values, requested) {
  pick(values): value {
    if value == requested {
      return true
    }
  }
  return false
}

func structlower.field_name(field) {
  VTime kind = parser.node_kind(field)
  if kind == parser.NODE_DECL {
    return get(field, 2)
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    return get(field, 2)
  }
  return get(field, 1)
}

func structlower.field_value(field) {
  VTime kind = parser.node_kind(field)
  if kind == parser.NODE_DECL {
    return get(field, 3)
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    return get(field, 3)
  }
  return get(field, 2)
}

func structlower.find_field(struct_node, name) {
  pick(structlower.fields(struct_node)): field {
    if structlower.field_name(field) == name {
      return field
    }
  }
  return NONE
}

func structlower.field_struct(structs, field) {
  if parser.node_kind(field) != parser.NODE_CLASS_INSTANCE {
    return NONE
  }
  VTime declared = get(field, 1)
  if structlower.find(structs, declared) == NONE {
    return NONE
  }
  return declared
}

func structlower.constructor_name(name) {
  return join(["aplstruct.new.", name], "")
}

func structlower.validator_name(name) {
  return join(["aplstruct.require.", name], "")
}

func structlower.getter_name(name, field) {
  return join(["aplstruct.get.", name, ".", field], "")
}

func structlower.setter_name(name, field) {
  return join(["aplstruct.set.", name, ".", field], "")
}

func structlower.env_get(env, name) {
  VTime index = len(env) - 1
  while (index >= 0) (-1) {
    VTime entry = get(env, index)
    if get(entry, 0) == name {
      return get(entry, 1)
    }
    index -= 1
  }
  return NONE
}

func structlower.env_put(env, name, struct_name) {
  VTime result = []
  pick(env): entry {
    if get(entry, 0) != name {
      add(result, entry)
    }
  }
  add(result, [name, struct_name])
  return result
}

func structlower.surface_field(field) {
  VTime kind = parser.node_kind(field)
  if kind == parser.NODE_DECL {
    return [kind, get(field, 1), get(field, 2), get(field, 3)]
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    return [kind, get(field, 1), get(field, 2), get(field, 3)]
  }
  return [kind, get(field, 1), get(field, 2)]
}

func structlower.surface(node) {
  VTime fields = []
  pick(structlower.fields(node)): field {
    add(fields, structlower.surface_field(field))
  }
  return [parser.NODE_STRUCT, structlower.name(node), fields]
}

func structlower.surface_is_valid(surface) {
  if len(surface) != 3 {
    return false
  }
  if parser.node_kind(surface) != parser.NODE_STRUCT {
    return false
  }
  if verifier.is_str(structlower.name(surface)) != true {
    return false
  }
  if contains(structlower.name(surface), ".") {
    return false
  }
  VTime names = []
  pick(structlower.fields(surface)): field {
    VTime kind = parser.node_kind(field)
    if kind == parser.NODE_DECL {
      if len(field) != 4 {
        return false
      }
      if parser.is_decl_keyword(get(field, 1)) != true {
        return false
      }
    } else if kind == parser.NODE_LIST_DECL {
      if len(field) != 3 {
        return false
      }
    } else if kind == parser.NODE_CLASS_INSTANCE {
      if len(field) != 4 {
        return false
      }
      if verifier.is_str(get(field, 1)) != true {
        return false
      }
    } else {
      return false
    }
    VTime field_name = structlower.field_name(field)
    if verifier.is_str(field_name) != true {
      return false
    }
    if contains(field_name, ".") {
      return false
    }
    if structlower.includes(names, field_name) {
      return false
    }
    add(names, field_name)
  }
  return true
}

func structlower.validate_struct(structs, node) {
  VTime name = structlower.name(node)
  if contains(name, ".") {
    return structlower.fail("struct name must be a simple identifier", node)
  }
  VTime names = []
  pick(structlower.fields(node)): field {
    VTime field_name = structlower.field_name(field)
    if contains(field_name, ".") {
      return structlower.fail("struct field name must be a simple identifier", field)
    }
    if structlower.includes(names, field_name) {
      return structlower.fail(join(["duplicate field `", field_name, "` in struct `", name, "`"], ""), field)
    }
    add(names, field_name)
    if parser.node_kind(field) == parser.NODE_CLASS_INSTANCE {
      VTime declared = get(field, 1)
      if declared == name {
        return structlower.fail("struct cannot contain itself by value", field)
      }
    }
  }
  return structlower.ok(node)
}

func structlower.has_value_cycle(structs, name, path) {
  if structlower.includes(path, name) {
    return true
  }

  VTime next_path = path[:]
  add(next_path, name)
  VTime node = structlower.find(structs, name)
  if node == NONE {
    return false
  }

  pick(structlower.fields(node)): field {
    VTime nested = structlower.field_struct(structs, field)
    if nested != NONE {
      if structlower.has_value_cycle(structs, nested, next_path) {
        return true
      }
    }
  }
  return false
}

func structlower.expr_struct(expression, env, structs) {
  VTime kind = parser.expr_kind(expression)
  if kind == parser.EXPR_CALL {
    VTime called = get(expression, 1)
    if structlower.find(structs, called) != NONE {
      return called
    }
    return NONE
  }
  if kind != parser.EXPR_VAR {
    return NONE
  }
  VTime parts = split(get(expression, 1), ".")
  VTime current = structlower.env_get(env, get(parts, 0))
  VTime index = 1
  while ((current != NONE) and (index < len(parts))) (-1) {
    VTime field = structlower.find_field(structlower.find(structs, current), get(parts, index))
    if field == NONE {
      return NONE
    }
    current = structlower.field_struct(structs, field)
    index += 1
  }
  return current
}

func structlower.rewrite_exprs(expressions, env, structs) {
  VTime result = []
  pick(expressions): expression {
    add(result, structlower.rewrite_expr(expression, env, structs))
  }
  return result
}

func structlower.rewrite_optional_expr(expression, env, structs) {
  if expression == NONE {
    return NONE
  }
  return structlower.rewrite_expr(expression, env, structs)
}

func structlower.rewrite_expr(expression, env, structs) {
  VTime kind = parser.expr_kind(expression)
  if kind == parser.EXPR_VAR {
    VTime parts = split(get(expression, 1), ".")
    VTime current_struct = structlower.env_get(env, get(parts, 0))
    if (current_struct == NONE) or (len(parts) < 2) {
      return expression
    }
    VTime current_expr = [parser.EXPR_VAR, get(parts, 0)]
    VTime index = 1
    while (index < len(parts)) (-1) {
      VTime field = structlower.find_field(structlower.find(structs, current_struct), get(parts, index))
      if field == NONE {
        return expression
      }
      current_expr = [parser.EXPR_CALL, structlower.getter_name(current_struct, get(parts, index)), [current_expr]]
      current_struct = structlower.field_struct(structs, field)
      index += 1
      if (index < len(parts)) and (current_struct == NONE) {
        return expression
      }
    }
    return current_expr
  }
  if kind == parser.EXPR_CALL {
    VTime name = get(expression, 1)
    VTime args = structlower.rewrite_exprs(get(expression, 2), env, structs)
    if structlower.find(structs, name) != NONE {
      return [kind, structlower.constructor_name(name), args]
    }
    return [kind, name, args]
  }
  if kind == parser.EXPR_UNARY {
    return [kind, get(expression, 1), structlower.rewrite_expr(get(expression, 2), env, structs)]
  }
  if kind == parser.EXPR_BINARY {
    return [kind, get(expression, 1), structlower.rewrite_expr(get(expression, 2), env, structs), structlower.rewrite_expr(get(expression, 3), env, structs)]
  }
  if kind == parser.EXPR_LIST {
    return [kind, structlower.rewrite_exprs(get(expression, 1), env, structs)]
  }
  if kind == parser.EXPR_INDEX {
    return [kind, structlower.rewrite_expr(get(expression, 1), env, structs), structlower.rewrite_expr(get(expression, 2), env, structs)]
  }
  if kind == parser.EXPR_SLICE {
    return [kind, structlower.rewrite_expr(get(expression, 1), env, structs), structlower.rewrite_optional_expr(get(expression, 2), env, structs), structlower.rewrite_optional_expr(get(expression, 3), env, structs), structlower.rewrite_optional_expr(get(expression, 4), env, structs)]
  }
  if kind == parser.EXPR_TAG {
    return [kind, structlower.rewrite_expr(get(expression, 1), env, structs), get(expression, 2)]
  }
  return expression
}

func structlower.update_path(structs, struct_name, target, fields, index, op, replacement, statement) {
  VTime struct_node = structlower.find(structs, struct_name)
  VTime field_name = get(fields, index)
  VTime field = structlower.find_field(struct_node, field_name)
  if field == NONE {
    return [false, join(["unknown field `", field_name, "` in struct `", struct_name, "`"], ""), statement]
  }
  VTime current = [parser.EXPR_CALL, structlower.getter_name(struct_name, field_name), [target]]
  VTime value = replacement
  if index + 1 < len(fields) {
    VTime nested = structlower.field_struct(structs, field)
    if nested == NONE {
      return [false, join(["field `", field_name, "` in struct `", struct_name, "` is not a struct"], ""), statement]
    }
    VTime nested_report = structlower.update_path(structs, nested, current, fields, index + 1, op, replacement, statement)
    if get(nested_report, 0) != true {
      return nested_report
    }
    value = get(nested_report, 1)
  } else if op != "=" {
    value = [parser.EXPR_BINARY, op[:len(op) - 1], current, replacement]
  }
  return [true, [parser.EXPR_CALL, structlower.setter_name(struct_name, field_name), [target, value]]]
}

func structlower.rewrite_class(node, structs) {
  VTime fields = []
  pick(get(node, 4)): field {
    VTime kind = parser.node_kind(field)
    if kind == parser.NODE_DECL {
      add(fields, [kind, get(field, 1), get(field, 2), structlower.rewrite_expr(get(field, 3), [], structs)])
    } else if kind == parser.NODE_CLASS_INSTANCE {
      add(fields, [kind, get(field, 1), get(field, 2), structlower.rewrite_expr(get(field, 3), [], structs)])
    } else {
      add(fields, [kind, get(field, 1), structlower.rewrite_expr(get(field, 2), [], structs)])
    }
  }
  VTime methods = []
  pick(get(node, 5)): entry {
    VTime method = get(entry, 1)
    VTime body_report = structlower.rewrite_block(get(method, 3), [], structs)
    if get(body_report, 0) != true {
      return body_report
    }
    add(methods, [get(entry, 0), [parser.NODE_FUNC, get(method, 1), get(method, 2), get(body_report, 1)]])
  }
  return [true, [parser.NODE_CLASS, get(node, 1), get(node, 2), get(node, 3), fields, methods]]
}

func structlower.rewrite_block(statements, env, structs) {
  VTime result = []
  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if kind == parser.NODE_CLASS_INSTANCE {
      VTime declared = get(statement, 1)
      if structlower.find(structs, declared) != NONE {
        VTime actual = structlower.expr_struct(get(statement, 3), env, structs)
        if (actual != NONE) and (actual != declared) {
          return [false, join(["struct `", actual, "` is not compatible with `", declared, "`"], ""), statement]
        }
        VTime initial = structlower.rewrite_expr(get(statement, 3), env, structs)
        if actual == NONE {
          initial = [parser.EXPR_CALL, structlower.validator_name(declared), [initial]]
        }
        add(result, [parser.NODE_VTIME_DECL, get(statement, 2), initial])
        env = structlower.env_put(env, get(statement, 2), declared)
        continue
      }
      add(result, statement)
      continue
    }

    if kind == parser.NODE_VTIME_DECL {
      VTime initial_struct = structlower.expr_struct(get(statement, 2), env, structs)
      add(result, [kind, get(statement, 1), structlower.rewrite_expr(get(statement, 2), env, structs)])
      if initial_struct != NONE {
        env = structlower.env_put(env, get(statement, 1), initial_struct)
      }
      continue
    }

    if kind == parser.NODE_ASSIGN {
      VTime name = get(statement, 1)
      VTime parts = split(name, ".")
      VTime root_struct = structlower.env_get(env, get(parts, 0))
      if (root_struct != NONE) and (len(parts) >= 2) {
        VTime fields = parts[1:]
        VTime replacement = structlower.rewrite_expr(get(statement, 3), env, structs)
        VTime update = structlower.update_path(structs, root_struct, [parser.EXPR_VAR, get(parts, 0)], fields, 0, get(statement, 2), replacement, statement)
        if get(update, 0) != true {
          return update
        }
        add(result, [parser.NODE_ASSIGN, get(parts, 0), "=", get(update, 1)])
        continue
      }
      VTime actual = structlower.expr_struct(get(statement, 3), env, structs)
      VTime expected = structlower.env_get(env, name)
      if (actual != NONE) and (expected != NONE) {
        if actual != expected {
          return [false, join(["struct `", actual, "` is not compatible with `", expected, "`"], ""), statement]
        }
      }
      VTime assigned = structlower.rewrite_expr(get(statement, 3), env, structs)
      if (actual == NONE) and (expected != NONE) {
        assigned = [parser.EXPR_CALL, structlower.validator_name(expected), [assigned]]
      }
      add(result, [kind, name, get(statement, 2), assigned])
      if (actual != NONE) and (expected == NONE) {
        env = structlower.env_put(env, name, actual)
      }
      continue
    }

    if kind == parser.NODE_OUT {
      VTime output_struct = structlower.expr_struct(get(statement, 1), env, structs)
      if output_struct != NONE {
        return [false, join(["struct `", output_struct, "` has no implicit out representation"], ""), statement]
      }
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs)])
      continue
    }

    if kind == parser.NODE_CLASS {
      VTime rewritten_class = structlower.rewrite_class(statement, structs)
      if get(rewritten_class, 0) != true {
        return rewritten_class
      }
      add(result, get(rewritten_class, 1))
      continue
    }

    if (kind == parser.NODE_DECL) or (kind == parser.NODE_LIST_DECL) {
      if kind == parser.NODE_DECL {
        add(result, [kind, get(statement, 1), get(statement, 2), structlower.rewrite_expr(get(statement, 3), env, structs)])
      } else {
        add(result, [kind, get(statement, 1), structlower.rewrite_expr(get(statement, 2), env, structs)])
      }
      continue
    }
    if kind == parser.NODE_EXPR {
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs)])
      continue
    }
    if (kind == parser.NODE_RETURN) or (kind == parser.NODE_FAIL) {
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs)])
      continue
    }
    if kind == parser.NODE_STOP {
      add(result, [kind, structlower.rewrite_optional_expr(get(statement, 1), env, structs)])
      continue
    }
    if kind == parser.NODE_ERR {
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs), structlower.rewrite_expr(get(statement, 2), env, structs)])
      continue
    }
    if kind == parser.NODE_FUNC {
      VTime body_report = structlower.rewrite_block(get(statement, 3), [], structs)
      if get(body_report, 0) != true {
        return body_report
      }
      add(result, [kind, get(statement, 1), get(statement, 2), get(body_report, 1)])
      continue
    }
    if kind == parser.NODE_IF {
      VTime body = structlower.rewrite_block(get(statement, 2), env[:], structs)
      if get(body, 0) != true {
        return body
      }
      VTime otherwise = structlower.rewrite_block(get(statement, 3), env[:], structs)
      if get(otherwise, 0) != true {
        return otherwise
      }
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs), get(body, 1), get(otherwise, 1)])
      continue
    }
    if kind == parser.NODE_WHILE {
      VTime body = structlower.rewrite_block(get(statement, 3), env[:], structs)
      if get(body, 0) != true {
        return body
      }
      VTime limited = structlower.rewrite_block(get(statement, 4), env[:], structs)
      if get(limited, 0) != true {
        return limited
      }
      VTime otherwise = structlower.rewrite_block(get(statement, 5), env[:], structs)
      if get(otherwise, 0) != true {
        return otherwise
      }
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs), structlower.rewrite_expr(get(statement, 2), env, structs), get(body, 1), get(limited, 1), get(otherwise, 1)])
      continue
    }
    if kind == parser.NODE_MATCH {
      VTime cases = []
      pick(get(statement, 2)): case_entry {
        VTime patterns = []
        pick(get(case_entry, 0)): pattern {
          if get(pattern, 0) == parser.MATCH_RANGE {
            add(patterns, [parser.MATCH_RANGE, structlower.rewrite_expr(get(pattern, 1), env, structs), structlower.rewrite_expr(get(pattern, 2), env, structs)])
          } else {
            add(patterns, [parser.MATCH_VALUE, structlower.rewrite_expr(get(pattern, 1), env, structs)])
          }
        }
        VTime case_body = structlower.rewrite_block(get(case_entry, 1), env[:], structs)
        if get(case_body, 0) != true {
          return case_body
        }
        add(cases, [patterns, get(case_body, 1)])
      }
      VTime default_body = structlower.rewrite_block(get(statement, 3), env[:], structs)
      if get(default_body, 0) != true {
        return default_body
      }
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs), cases, get(default_body, 1)])
      continue
    }
    if kind == parser.NODE_PICK {
      VTime body = structlower.rewrite_block(get(statement, 3), env[:], structs)
      if get(body, 0) != true {
        return body
      }
      add(result, [kind, structlower.rewrite_expr(get(statement, 1), env, structs), get(statement, 2), get(body, 1)])
      continue
    }
    if kind == parser.NODE_STRUCT {
      return [false, "struct declarations are only allowed at top level", statement]
    }
    add(result, statement)
  }
  return [true, result, env]
}

func structlower.constructor_function(structs, node) {
  VTime body = []
  VTime values = []
  VTime index = 0
  pick(structlower.fields(node)): field {
    VTime local = join(["__apl_struct_field_", structlower.name(node), "_", str(index)], "")
    VTime value = structlower.rewrite_expr(structlower.field_value(field), [], structs)
    VTime kind = parser.node_kind(field)
    if kind == parser.NODE_DECL {
      add(body, [kind, get(field, 1), local, value])
    } else if kind == parser.NODE_LIST_DECL {
      add(body, [kind, local, value])
    } else {
      add(body, [parser.NODE_VTIME_DECL, local, value])
    }
    add(values, [parser.EXPR_VAR, local])
    index += 1
  }
  VTime record = [parser.EXPR_LIST, [[parser.EXPR_STR, structlower.MAGIC], [parser.EXPR_STR, structlower.name(node)], [parser.EXPR_LIST, values]]]
  add(body, [parser.NODE_RETURN, record])
  return [parser.NODE_FUNC, structlower.constructor_name(structlower.name(node)), [], body]
}

func structlower.validator_function(node) {
  VTime value = [parser.EXPR_VAR, "value"]
  VTime invalid_length = [parser.EXPR_BINARY, "!=", [parser.EXPR_CALL, "std::len", [value]], [parser.EXPR_INT, 3]]
  VTime invalid_magic = [parser.EXPR_BINARY, "!=", [parser.EXPR_CALL, "std::get", [value, [parser.EXPR_INT, 0]]], [parser.EXPR_STR, structlower.MAGIC]]
  VTime invalid_type = [parser.EXPR_BINARY, "!=", [parser.EXPR_CALL, "std::get", [value, [parser.EXPR_INT, 1]]], [parser.EXPR_STR, structlower.name(node)]]
  VTime message = [parser.EXPR_STR, join(["expected struct `", structlower.name(node), "`"], "")]
  VTime body = [
    [parser.NODE_IF, invalid_length, [[parser.NODE_FAIL, message]], []],
    [parser.NODE_IF, invalid_magic, [[parser.NODE_FAIL, message]], []],
    [parser.NODE_IF, invalid_type, [[parser.NODE_FAIL, message]], []],
    [parser.NODE_RETURN, value]
  ]
  return [parser.NODE_FUNC, structlower.validator_name(structlower.name(node)), ["value"], body]
}

func structlower.getter_function(node, field, index) {
  VTime values = [parser.EXPR_CALL, "std::get", [[parser.EXPR_VAR, "self"], [parser.EXPR_INT, 2]]]
  VTime value = [parser.EXPR_CALL, "std::get", [values, [parser.EXPR_INT, index]]]
  return [parser.NODE_FUNC, structlower.getter_name(structlower.name(node), structlower.field_name(field)), ["self"], [[parser.NODE_RETURN, value]]]
}

func structlower.setter_function(node, field, index) {
  VTime checked = join(["__apl_struct_checked_", structlower.name(node), "_", structlower.field_name(field)], "")
  VTime body = []
  VTime kind = parser.node_kind(field)
  if kind == parser.NODE_DECL {
    add(body, [kind, get(field, 1), checked, [parser.EXPR_VAR, "value"]])
  } else if kind == parser.NODE_LIST_DECL {
    add(body, [kind, checked, [parser.EXPR_VAR, "value"]])
  } else {
    add(body, [parser.NODE_VTIME_DECL, checked, [parser.EXPR_VAR, "value"]])
  }
  VTime values_name = join(["__apl_struct_values_", structlower.name(node), "_", structlower.field_name(field)], "")
  VTime source_values = [parser.EXPR_CALL, "std::get", [[parser.EXPR_VAR, "self"], [parser.EXPR_INT, 2]]]
  add(body, [parser.NODE_LIST_DECL, values_name, [parser.EXPR_SLICE, source_values, NONE, NONE, NONE]])
  add(body, [parser.NODE_EXPR, [parser.EXPR_CALL, "std::set", [[parser.EXPR_VAR, values_name], [parser.EXPR_INT, index], [parser.EXPR_VAR, checked]]]])
  VTime record = [parser.EXPR_LIST, [[parser.EXPR_STR, structlower.MAGIC], [parser.EXPR_STR, structlower.name(node)], [parser.EXPR_VAR, values_name]]]
  add(body, [parser.NODE_RETURN, record])
  return [parser.NODE_FUNC, structlower.setter_name(structlower.name(node), structlower.field_name(field)), ["self", "value"], body]
}

func structlower.lower(statements, imported_structs) {
  VTime local_structs = []
  VTime ordinary = []
  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_STRUCT {
      if structlower.find(local_structs, structlower.name(statement)) != NONE {
        return structlower.fail(join(["duplicate struct `", structlower.name(statement), "`"], ""), statement)
      }
      add(local_structs, statement)
    } else {
      add(ordinary, statement)
    }
  }

  VTime structs = []
  pick(imported_structs): imported {
    if structlower.surface_is_valid(imported) != true {
      return structlower.fail("invalid imported struct surface", imported)
    }
    if structlower.find(structs, structlower.name(imported)) != NONE {
      return structlower.fail(join(["duplicate imported struct `", structlower.name(imported), "`"], ""), imported)
    }
    add(structs, imported)
  }
  pick(local_structs): node {
    if structlower.find(structs, structlower.name(node)) != NONE {
      return structlower.fail(join(["duplicate struct `", structlower.name(node), "`"], ""), node)
    }
    add(structs, node)
  }

  pick(local_structs): node {
    VTime validation = structlower.validate_struct(structs, node)
    if get(validation, 0) != true {
      return validation
    }
    if structlower.has_value_cycle(structs, structlower.name(node), []) {
      return structlower.fail(join(["struct `", structlower.name(node), "` has cyclic value composition"], ""), node)
    }
  }

  VTime generated = []
  pick(local_structs): node {
    add(generated, structlower.constructor_function(structs, node))
    add(generated, structlower.validator_function(node))
    VTime index = 0
    pick(structlower.fields(node)): field {
      add(generated, structlower.getter_function(node, field, index))
      add(generated, structlower.setter_function(node, field, index))
      index += 1
    }
  }

  VTime rewritten = structlower.rewrite_block(ordinary, [], structs)
  if get(rewritten, 0) != true {
    return rewritten
  }
  pick(get(rewritten, 1)): statement {
    add(generated, statement)
  }

  VTime surfaces = []
  pick(local_structs): node {
    add(surfaces, structlower.surface(node))
  }
  return [true, generated, surfaces]
}
