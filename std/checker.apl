# APL checker bootstrap helpers.
# This module validates semantic rules that the bootstrap runtime can enforce
# before lowering AST into IR.

AVStr checker.STATUS_OK = "OK"
AVStr checker.STATUS_FAIL = "FAIL"
List checker.STANDARD_EXPORTS = ["pick", "add", "set", "get", "pop", "int", "float", "bool", "str", "bytes", "json", "split", "join", "len", "contains", "ord", "char", "pow", "timer_start", "timer_stop", "timer_elapsed", "now_utc", "sha1", "sha256", "sha512", "sha3_384", "windows_available", "windows_load_library", "windows_get_procedure", "windows_call", "windows_close_library", "windows_last_error", "windows_utf16", "windows_message_box", "windows_create_window", "windows_show_window", "windows_destroy_window", "windows_run_message_loop", "windows_pump_messages", "windows_next_event", "windows_create_control", "windows_set_window_text", "windows_window_text", "windows_move_window", "windows_selection", "windows_set_selection", "windows_replace_selection", "windows_line_count", "windows_set_text_color", "windows_find_text", "windows_set_background_color", "windows_set_text_bold", "windows_can_undo", "windows_undo", "windows_can_redo", "windows_clear_history", "windows_event_mask", "windows_set_event_mask", "windows_first_visible_line", "windows_scroll_lines", "windows_focus", "windows_set_text_font", "windows_run_process", "windows_open_file_dialog", "windows_save_file_dialog", "windows_create_menu", "windows_append_menu_item", "windows_append_submenu", "windows_append_menu_separator", "windows_set_menu", "windows_set_menu_item_enabled", "windows_set_menu_item_checked", "windows_destroy_menu", "windows_wait_event", "windows_load_icon", "windows_set_window_icon", "windows_destroy_icon", "windows_set_deferred_close", "windows_create_accelerators", "windows_set_accelerators", "windows_destroy_accelerators", "windows_start_process", "windows_poll_process", "windows_wait_process", "windows_terminate_process"]

func checker.entry_name(entry) {
  return get(entry, 0)
}

func checker.entry_role(entry) {
  return get(entry, 1)
}

func checker.entry_type(entry) {
  return get(entry, 2)
}

func checker.decl_protection(typ) {
  if contains(typ, "SASV") {
    return "SASV"
  }

  if contains(typ, "ASV") {
    return "ASV"
  }

  return "AV"
}

func checker.entry_protection(entry) {
  if len(entry) >= 4 {
    return get(entry, 3)
  }

  if checker.entry_role(entry) == "Absolute" {
    return checker.decl_protection(checker.entry_type(entry))
  }

  return "AV"
}

func checker.entry_list_alias(entry) {
  if len(entry) >= 5 {
    return get(entry, 4)
  }

  return NONE
}

func checker.protection_rank(protection) {
  if protection == "SASV" {
    return 2
  }

  if protection == "ASV" {
    return 1
  }

  return 0
}

func checker.max_protection(left, right) {
  if checker.protection_rank(left) >= checker.protection_rank(right) {
    return left
  }

  return right
}

func checker.name_exists(names, name) {
  pick(names): existing {
    if checker.entry_role(existing) != "Excluded" {
      if checker.entry_name(existing) == name {
        return true
      }
    }
  }

  return false
}

func checker.find_name(names, name) {
  VTime index = len(names) - 1

  while (index >= 0) (-1) {
    VTime existing = get(names, index)

    if checker.entry_role(existing) != "Excluded" {
      if checker.entry_name(existing) == name {
        return existing
      }
    }

    index -= 1
  }

  return NONE
}

func checker.find_function(names, name) {
  pick(names): existing {
    if checker.entry_name(existing) == name {
      if checker.entry_role(existing) == "Func" {
        return existing
      }
    }
  }

  return NONE
}

func checker.value_exists(values, value) {
  pick(values): existing {
    if existing == value {
      return true
    }
  }

  return false
}

func checker.is_ascii_letter(ch) {
  VTime code = ord(ch)

  if (code >= 65) and (code <= 90) {
    return true
  }

  return (code >= 97) and (code <= 122)
}

func checker.name_part_is_valid(part) {
  if len(part) == 0 {
    return false
  }

  VTime first = part[0]

  if first != "_" {
    if checker.is_ascii_letter(first) != true {
      return false
    }
  }

  VTime index = 1

  while (index < len(part)) (-1) {
    VTime ch = part[index]

    if ch != "_" {
      if checker.is_ascii_letter(ch) != true {
        if lexer.is_digit(ch) != true {
          return false
        }
      }
    }

    index += 1
  }

  return true
}

func checker.name_is_valid(name) {
  if len(name) == 0 {
    return false
  }

  if lexer.is_keyword(name) {
    return false
  }

  if checker.builtin_arity(name) >= 0 {
    if checker.is_memory_builtin(name) != true {
      return false
    }
  }

  if name[0] == "." {
    return false
  }

  if name[len(name) - 1] == "." {
    return false
  }

  if contains(name, "..") {
    return false
  }

  pick(split(name, ".")): part {
    if checker.name_part_is_valid(part) != true {
      return false
    }
  }

  return true
}

func checker.composition_name_is_valid(name) {
  if len(name) == 0 {
    return false
  }

  if name[0] == "." {
    return false
  }

  if name[len(name) - 1] == "." {
    return false
  }

  if contains(name, "..") {
    return false
  }

  pick(split(name, ".")): part {
    if checker.name_part_is_valid(part) != true {
      return false
    }
  }

  return true
}

func checker.exclusion_exists(exclusions, library, symbol) {
  pick(exclusions): exclusion {
    if (get(exclusion, 0) == library) and (get(exclusion, 1) == symbol) {
      return true
    }
  }

  return false
}

func checker.library_entry(libraries, name) {
  pick(libraries): library {
    if get(library, 0) == name {
      return library
    }
  }

  return NONE
}

func checker.library_has_symbol(library, name) {
  pick(get(library, 1)): symbol {
    if get(symbol, 0) == name {
      return true
    }
  }

  return false
}

func checker.library_symbols_for_exclusion(library, requested) {
  VTime matches = []

  pick(get(library, 1)): symbol {
    VTime full_name = get(symbol, 0)
    VTime parts = split(full_name, ".")
    VTime public_name = get(parts, len(parts) - 1)

    if public_name == requested {
      add(matches, full_name)
    }
  }

  return matches
}

func checker.collect_composition_with_libraries(statements, libraries) {
  VTime includes = []
  VTime exclusions = []
  VTime namespaces = []
  VTime checked_statements = []

  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_INCLUDE {
      VTime library = get(statement, 1)

      if checker.composition_name_is_valid(library) != true {
        return checker.fail_at_node([], join(["invalid name `", library, "`"], ""), statement)
      }

      if (library != "base") and (library != "std") {
        if checker.library_entry(libraries, library) == NONE {
          return checker.fail_at_node([], join(["unknown library `", library, "`"], ""), statement)
        }
      }

      if checker.value_exists(includes, library) {
        return checker.fail_at_node([], join(["duplicate include `", library, "`"], ""), statement)
      }

      add(includes, library)
    }
  }

  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_EXCLUDE {
      VTime symbol = get(statement, 1)
      VTime library = get(statement, 2)

      if checker.composition_name_is_valid(library) != true {
        return checker.fail_at_node([], join(["invalid name `", library, "`"], ""), statement)
      }

      if checker.name_part_is_valid(symbol) != true {
        return checker.fail_at_node([], join(["invalid public symbol `", symbol, "`"], ""), statement)
      }

      if (library == "base") or (library == "std") {
        if checker.base_export_is_known(symbol) != true {
          return checker.fail_at_node([], join(["unknown symbol `", symbol, "` in library `std`"], ""), statement)
        }
      } else {
        VTime library_entry = checker.library_entry(libraries, library)

        if library_entry == NONE {
          return checker.fail_at_node([], join(["unknown library `", library, "`"], ""), statement)
        }

        if checker.value_exists(includes, library) != true {
          return checker.fail_at_node([], join(["library `", library, "` is not included"], ""), statement)
        }

        VTime matches = checker.library_symbols_for_exclusion(library_entry, symbol)

        if len(matches) == 0 {
          return checker.fail_at_node([], join(["unknown symbol `", symbol, "` in library `", library, "`"], ""), statement)
        }

        if len(matches) > 1 {
          return checker.fail_at_node([], join(["ambiguous symbol `", symbol, "` in library `", library, "`"], ""), statement)
        }

        symbol = get(matches, 0)
      }

      if checker.exclusion_exists(exclusions, library, symbol) {
        return checker.fail_at_node([], join(["duplicate exclude `", symbol, "` from library `", library, "`"], ""), statement)
      }

      add(exclusions, [library, symbol])
    }
  }

  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if (kind == parser.NODE_USING) or (kind == parser.NODE_NC) {
      if len(namespaces) != 0 {
        return checker.fail_at_node([], "only one using namespace or nc directive is allowed", statement)
      }

      VTime requested = []

      if kind == parser.NODE_USING {
        add(requested, get(statement, 1))
      } else {
        requested = get(statement, 1)

        if len(requested) < 2 {
          return checker.fail_at_node([], "nc requires at least two namespaces", statement)
        }
      }

      pick(requested): namespace_name {
        if checker.composition_name_is_valid(namespace_name) != true {
          return checker.fail_at_node([], join(["invalid namespace `", namespace_name, "`"], ""), statement)
        }

        if checker.value_exists(namespaces, namespace_name) {
          return checker.fail_at_node([], join(["duplicate namespace `", namespace_name, "`"], ""), statement)
        }

        if (namespace_name != "std") and (namespace_name != "base") {
          if checker.value_exists(includes, namespace_name) != true {
            return checker.fail_at_node([], join(["namespace `", namespace_name, "` is not included"], ""), statement)
          }
        }

        add(namespaces, namespace_name)
      }
    }
  }

  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if (((kind != parser.NODE_INCLUDE) and (kind != parser.NODE_EXCLUDE)) and (kind != parser.NODE_USING)) and (kind != parser.NODE_NC) {
      add(checked_statements, statement)
    }
  }

  return [checker.STATUS_OK, checked_statements, includes, exclusions, namespaces]
}

func checker.collect_composition(statements) {
  return checker.collect_composition_with_libraries(statements, [])
}

func checker.validate_name(names, name) {
  if name == "std::main" {
    return checker.ok(names)
  }

  if checker.name_is_valid(name) {
    return checker.ok(names)
  }

  return checker.fail(names, join(["invalid name `", name, "`"], ""))
}

func checker.is_main_implementation(name) {
  if name == "std::main" {
    return true
  }
  VTime parts = split(name, ".")
  if len(parts) != 3 {
    return false
  }
  return (get(parts, 0) == "aplraw") and (get(parts, 1) == "main")
}

func checker.add_name(names, name, role, typ) {
  VTime protection = "AV"

  if role == "Absolute" {
    protection = checker.decl_protection(typ)
  }

  add(names, [name, role, typ, protection])
  return names
}

func checker.add_name_with_protection(names, name, role, typ, protection) {
  add(names, [name, role, typ, protection, NONE])
  return names
}

func checker.update_name_protection(names, name, protection) {
  VTime updated = []
  VTime target_index = -1
  VTime index = 0

  pick(names): entry {
    if checker.entry_name(entry) == name {
      target_index = index
    }

    index += 1
  }

  index = 0

  pick(names): entry {
    if index == target_index {
      add(updated, [checker.entry_name(entry), checker.entry_role(entry), checker.entry_type(entry), protection, checker.entry_list_alias(entry)])
    } else {
      add(updated, entry)
    }

    index += 1
  }

  return updated
}

func checker.update_name_list_alias(names, name, alias) {
  VTime updated = []

  pick(names): entry {
    if checker.entry_name(entry) == name {
      add(updated, [checker.entry_name(entry), checker.entry_role(entry), checker.entry_type(entry), checker.entry_protection(entry), alias])
    } else {
      add(updated, entry)
    }
  }

  return updated
}

func checker.expr_list_alias(expression, names) {
  VTime kind = parser.expr_kind(expression)

  if kind != parser.EXPR_VAR {
    return NONE
  }

  VTime entry = checker.find_name(names, parser.expr_value(expression))
  if entry == NONE {
    return NONE
  }

  if checker.entry_role(entry) == "List" {
    return checker.entry_name(entry)
  }

  if checker.entry_role(entry) == "VTime" {
    return checker.entry_list_alias(entry)
  }

  return NONE
}

func checker.raise_list_alias_protection(names, list_name, protection) {
  VTime updated = []

  pick(names): entry {
    VTime aliases_list = false
    if checker.entry_role(entry) == "List" {
      aliases_list = checker.entry_name(entry) == list_name
    } else if checker.entry_role(entry) == "VTime" {
      aliases_list = checker.entry_list_alias(entry) == list_name
    }

    if aliases_list {
      VTime raised = checker.max_protection(checker.entry_protection(entry), protection)
      add(updated, [checker.entry_name(entry), checker.entry_role(entry), checker.entry_type(entry), raised, checker.entry_list_alias(entry)])
    } else {
      add(updated, entry)
    }
  }

  return updated
}

func checker.declared_name(node) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_DECL {
    return get(node, 2)
  }

  if kind == parser.NODE_LIST_DECL {
    return get(node, 1)
  }

  if kind == parser.NODE_VTIME_DECL {
    return get(node, 1)
  }

  if kind == parser.NODE_FUNC {
    return get(node, 1)
  }

  return NONE
}

func checker.declared_role(node) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_DECL {
    return "Absolute"
  }

  if kind == parser.NODE_LIST_DECL {
    return "List"
  }

  if kind == parser.NODE_VTIME_DECL {
    return "VTime"
  }

  if kind == parser.NODE_FUNC {
    return "Func"
  }

  return NONE
}

func checker.declared_type(node) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_DECL {
    return get(node, 1)
  }

  if kind == parser.NODE_LIST_DECL {
    return "List"
  }

  if kind == parser.NODE_VTIME_DECL {
    return "VTime"
  }

  if kind == parser.NODE_FUNC {
    return len(get(node, 2))
  }

  return NONE
}

func checker.declared_protection(node, names) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_DECL {
    return checker.decl_protection(get(node, 1))
  }

  if kind == parser.NODE_LIST_DECL {
    return checker.expr_protection(get(node, 2), names)
  }

  if kind == parser.NODE_VTIME_DECL {
    return checker.expr_protection(get(node, 2), names)
  }

  return "AV"
}

func checker.fail(names, message) {
  return [checker.STATUS_FAIL, names, message]
}

func checker.attach_failure(state, node) {
  if get(state, 0) == checker.STATUS_FAIL {
    if len(state) < 4 {
      VTime token_index = parser.node_token_index(node)

      if token_index != NONE {
        add(state, token_index)
      }
    }
  }

  return state
}

func checker.fail_at_node(names, message, node) {
  return checker.attach_failure(checker.fail(names, message), node)
}

func checker.failure_report(state) {
  if len(state) >= 4 {
    if get(state, 3) == NONE {
      return [get(state, 0), get(state, 2)]
    }

    return [get(state, 0), get(state, 2), get(state, 3)]
  }

  return [get(state, 0), get(state, 2)]
}

func checker.ok(names) {
  return [checker.STATUS_OK, names, NONE]
}

func checker.merge_global_names(names, scoped_names) {
  VTime merged = []
  VTime index = 0

  pick(names): entry {
    if index >= len(scoped_names) {
      add(merged, entry)
    } else {
      add(merged, get(scoped_names, index))
    }

    index += 1
  }

  index = len(names)

  while (index < len(scoped_names)) (-1) {
    VTime entry = get(scoped_names, index)

    if checker.entry_role(entry) != "VTime" {
      add(merged, entry)
    }

    index += 1
  }

  return merged
}

func checker.merge_existing_names(names, scoped_names) {
  VTime merged = []
  VTime index = 0

  pick(names): entry {
    if index >= len(scoped_names) {
      add(merged, entry)
    } else {
      add(merged, get(scoped_names, index))
    }
    index += 1
  }

  return merged
}

func checker.validate_decl_name(node, names, allow_predeclared_func) {
  VTime name = checker.declared_name(node)
  VTime kind = parser.node_kind(node)

  if name == NONE {
    return checker.ok(names)
  }

  VTime valid_state = checker.validate_name(names, name)

  if get(valid_state, 0) != checker.STATUS_OK {
    return valid_state
  }

  if kind == parser.NODE_FUNC {
    VTime existing = checker.find_name(names, name)

    if existing != NONE {
      if (allow_predeclared_func == true) and (checker.entry_role(existing) == "Func") {
        return checker.ok(names)
      }
    }
  }

  if checker.name_exists(names, name) {
    return checker.fail(names, join(["duplicate name `", name, "`"], ""))
  }

  names = checker.add_name_with_protection(names, name, checker.declared_role(node), checker.declared_type(node), checker.declared_protection(node, names))
  return checker.ok(names)
}

func checker.validate_decl_collision(node, names, allow_predeclared_func) {
  VTime name = checker.declared_name(node)
  VTime kind = parser.node_kind(node)

  if name == NONE {
    return checker.ok(names)
  }

  VTime valid_state = checker.validate_name(names, name)

  if get(valid_state, 0) != checker.STATUS_OK {
    return valid_state
  }

  if kind == parser.NODE_FUNC {
    VTime existing = checker.find_name(names, name)

    if existing != NONE {
      if (allow_predeclared_func == true) and (checker.entry_role(existing) == "Func") {
        return checker.ok(names)
      }
    }
  }

  if checker.name_exists(names, name) {
    return checker.fail(names, join(["duplicate name `", name, "`"], ""))
  }

  return checker.ok(names)
}

func checker.require_name(names, name, message) {
  if checker.name_exists(names, name) {
    return checker.ok(names)
  }

  return checker.fail(names, join([message, " `", name, "`"], ""))
}

func checker.type_is_public_numeric(typ) {
  if typ == "AVInt" {
    return true
  }

  if typ == "AVFloat" {
    return true
  }

  return false
}

func checker.builtin_arity(name) {
  if (name == "memory.malloc") or (name == "raw_malloc") {
    return 1
  }
  if (name == "memory.load") or (name == "raw_load") {
    return 4
  }
  if (name == "memory.mark") or (name == "raw_mark") {
    return 1
  }
  if (name == "memory.free") or (name == "raw_free") {
    return 0
  }
  if (name == "memory.where") or (name == "raw_where") {
    return 2
  }
  if (name == "memory.read") or (name == "raw_read") {
    return 3
  }
  if (name == "memory.write") or (name == "raw_write") {
    return 3
  }
  if (name == "memory.call") or (name == "raw_call") {
    return 2
  }
  if (name == "memory._scope_enter") or (name == "raw_scope_enter") {
    return 1
  }
  if (name == "memory._scope_exit") or (name == "raw_scope_exit") {
    return 0
  }
  if name == "raw_scope_leave" {
    return 1
  }

  if name == "timer_start" {
    return 0
  }
  if name == "timer_stop" {
    return 0
  }
  if name == "timer_elapsed" {
    return 0
  }
  if name == "now_utc" {
    return 0
  }
  if (((name == "sha1") or (name == "sha256")) or (name == "sha512")) or (name == "sha3_384") {
    return 1
  }
  if ((((name == "windows_available") or (name == "windows_last_error")) or (name == "windows_run_message_loop")) or (name == "windows_pump_messages")) or (name == "windows_next_event") {
    return 0
  }
  if ((((name == "windows_load_library") or (name == "windows_close_library")) or (name == "windows_utf16")) or (name == "windows_show_window")) or (name == "windows_destroy_window") {
    return 1
  }
  if name == "windows_call" {
    return 2
  }
  if (name == "windows_message_box") or (name == "windows_create_window") {
    return 3
  }
  if name == "windows_get_procedure" {
    return 4
  }
  if name == "windows_create_control" {
    return 10
  }
  if name == "windows_set_window_text" {
    return 2
  }
  if name == "windows_window_text" {
    return 1
  }
  if name == "windows_move_window" {
    return 6
  }
  if (name == "windows_selection") or (name == "windows_line_count") {
    return 1
  }
  if (name == "windows_set_selection") or (name == "windows_replace_selection") {
    return 3
  }
  if name == "windows_set_text_color" {
    return 4
  }
  if name == "windows_find_text" {
    return 6
  }
  if ((name == "windows_set_background_color") or (name == "windows_set_text_bold")) {
    return 4
  }
  if name == "windows_set_text_font" {
    return 5
  }
  if (name == "windows_run_process") or (name == "windows_start_process") {
    return 4
  }
  if ((name == "windows_poll_process") or (name == "windows_wait_process")) or (name == "windows_terminate_process") {
    return 1
  }
  if (name == "windows_open_file_dialog") or (name == "windows_save_file_dialog") {
    return 7
  }
  if ((name == "windows_create_menu") or (name == "windows_append_menu_separator")) or (name == "windows_destroy_menu") {
    return 1
  }
  if name == "windows_set_menu" {
    return 2
  }
  if (((name == "windows_append_menu_item") or (name == "windows_append_submenu")) or (name == "windows_set_menu_item_enabled")) or (name == "windows_set_menu_item_checked") {
    return 3
  }
  if name == "windows_wait_event" {
    return 0
  }
  if name == "windows_destroy_icon" {
    return 1
  }
  if name == "windows_set_window_icon" {
    return 2
  }
  if name == "windows_load_icon" {
    return 3
  }
  if name == "windows_set_deferred_close" {
    return 2
  }
  if (name == "windows_create_accelerators") or (name == "windows_destroy_accelerators") {
    return 1
  }
  if name == "windows_set_accelerators" {
    return 2
  }
  if (name == "windows_can_undo") or (name == "windows_undo") {
    return 1
  }
  if (name == "windows_can_redo") or (name == "windows_redo") {
    return 1
  }
  if (name == "windows_clear_history") or (name == "windows_event_mask") {
    return 1
  }
  if (name == "windows_first_visible_line") or (name == "windows_focus") {
    return 1
  }
  if (name == "windows_set_event_mask") or (name == "windows_scroll_lines") {
    return 2
  }

  if name == "get" {
    return 2
  }

  if name == "split" {
    return 2
  }

  if name == "join" {
    return 2
  }

  if name == "contains" {
    return 2
  }

  if name == "pow" {
    return 2
  }

  if name == "add" {
    return 2
  }

  if name == "set" {
    return 3
  }

  if name == "len" {
    return 1
  }

  if name == "ord" {
    return 1
  }

  if name == "char" {
    return 1
  }

  if name == "pop" {
    return 1
  }

  if name == "int" {
    return 1
  }

  if name == "float" {
    return 1
  }

  if name == "bool" {
    return 1
  }

  if name == "str" {
    return 1
  }

  if name == "bytes" {
    return 1
  }

  if name == "json" {
    return 1
  }

  return -1
}

func checker.is_builtin_call(name) {
  return checker.builtin_arity(name) >= 0
}

func checker.is_memory_builtin(name) {
  return (contains(name, "memory.")) and (checker.builtin_arity(name) >= 0)
}

func checker.is_raw_memory_runtime(name) {
  return (contains(name, "raw_")) and (checker.builtin_arity(name) >= 0)
}

func checker.memory_runtime_name(name) {
  if name == "memory.malloc" {
    return "raw_malloc"
  }
  if name == "memory.load" {
    return "raw_load"
  }
  if name == "memory.mark" {
    return "raw_mark"
  }
  if name == "memory.free" {
    return "raw_free"
  }
  if name == "memory.where" {
    return "raw_where"
  }
  if name == "memory.read" {
    return "raw_read"
  }
  if name == "memory.write" {
    return "raw_write"
  }
  if name == "memory.call" {
    return "raw_call"
  }
  if name == "memory._scope_enter" {
    return "raw_scope_enter"
  }
  if name == "memory._scope_exit" {
    return "raw_scope_exit"
  }
  return name
}

func checker.base_export_is_known(name) {
  return checker.value_exists(checker.STANDARD_EXPORTS, name)
}

func checker.base_symbol_is_excluded(names, symbol) {
  pick(names): entry {
    if checker.entry_role(entry) == "Excluded" {
      if (checker.entry_type(entry) == "base") or (checker.entry_type(entry) == "std") {
        if checker.entry_name(entry) == symbol {
          return true
        }
      }
    }
  }

  return false
}

func checker.base_excluded_library(names, symbol) {
  pick(names): entry {
    if checker.entry_role(entry) == "Excluded" {
      if (checker.entry_type(entry) == "base") or (checker.entry_type(entry) == "std") {
        if checker.entry_name(entry) == symbol {
          return checker.entry_type(entry)
        }
      }
    }
  }

  return NONE
}

func checker.excluded_library_for_symbol(names, symbol) {
  pick(names): entry {
    if checker.entry_role(entry) == "Excluded" {
      if checker.entry_name(entry) == symbol {
        return checker.entry_type(entry)
      }
    }
  }

  return NONE
}

func checker.require_base_symbol(names, symbol) {
  VTime library = checker.base_excluded_library(names, symbol)

  if library != NONE {
    return checker.fail(names, join(["excluded symbol `", symbol, "` from library `", library, "`"], ""))
  }

  return checker.ok(names)
}

func checker.namespace_is_open(names, namespace_name) {
  pick(names): entry {
    if checker.entry_role(entry) == "Namespace" {
      if checker.entry_name(entry) == namespace_name {
        return true
      }
    }
  }

  return false
}

func checker.strict_std_enabled(names) {
  return checker.find_name(names, "@strict_std") != NONE
}

func checker.canonical_qualified_call(name) {
  if name == "std::main" {
    return name
  }

  VTime parts = split(name, "::")

  if len(parts) != 2 {
    return name
  }

  if get(parts, 0) == "std" {
    return get(parts, 1)
  }

  return join([get(parts, 0), ".", get(parts, 1)], "")
}

func checker.resolve_call_name(names, name) {
  if contains(name, "::") {
    return [checker.STATUS_OK, checker.canonical_qualified_call(name)]
  }

  if checker.find_function(names, name) != NONE {
    return [checker.STATUS_OK, name]
  }

  if checker.is_builtin_call(name) {
    if checker.strict_std_enabled(names) {
      if checker.is_raw_memory_runtime(name) != true {
        if (checker.namespace_is_open(names, "std") != true) and (checker.namespace_is_open(names, "base") != true) {
          return [checker.STATUS_FAIL, join(["standard function `", name, "` requires the std:: prefix"], "")]
        }
      }
    }

    return [checker.STATUS_OK, name]
  }

  VTime resolved = NONE
  VTime owners = []

  pick(names): namespace_entry {
    if checker.entry_role(namespace_entry) == "Namespace" {
      VTime namespace_name = checker.entry_name(namespace_entry)

      if (namespace_name != "std") and (namespace_name != "base") {
        VTime candidate = join([namespace_name, ".", name], "")

        if checker.find_function(names, candidate) != NONE {
          add(owners, namespace_name)
          resolved = candidate
        }
      }
    }
  }

  if len(owners) > 1 {
    return [checker.STATUS_FAIL, join(["namespace conflict for `", name, "`"], "")]
  }

  if len(owners) == 1 {
    return [checker.STATUS_OK, resolved]
  }

  return [checker.STATUS_OK, name]
}

func checker.decl_value_type(typ) {
  if contains(typ, "Float") {
    return "Float"
  }

  if contains(typ, "Bool") {
    return "Bool"
  }

  if contains(typ, "Bytes") {
    return "Bytes"
  }

  if contains(typ, "Json") {
    return "Json"
  }

  if contains(typ, "Int") {
    return "Int"
  }

  if contains(typ, "Str") {
    return "Str"
  }

  return "VTime"
}

func checker.entry_value_type(entry) {
  VTime role = checker.entry_role(entry)

  if role == "Absolute" {
    return checker.decl_value_type(checker.entry_type(entry))
  }

  if role == "List" {
    return "List"
  }

  return "VTime"
}

func checker.call_result_type(name, args, names) {
  name = checker.canonical_qualified_call(name)
  VTime runtime_name = checker.memory_runtime_name(name)

  if (runtime_name == "raw_malloc") or (runtime_name == "raw_load") {
    return "List"
  }
  if (runtime_name == "raw_mark") or (runtime_name == "raw_write") {
    return "Bool"
  }
  if runtime_name == "raw_scope_enter" {
    return "Bool"
  }
  if (runtime_name == "raw_free") or (runtime_name == "raw_call") {
    return "Int"
  }
  if runtime_name == "raw_scope_exit" {
    return "Int"
  }
  if runtime_name == "raw_scope_leave" {
    return checker.expr_type(get(args, 0), names)
  }
  if runtime_name == "raw_read" {
    return "Bytes"
  }

  if name == "timer_start" {
    return "Bool"
  }

  if name == "timer_stop" {
    return "Bool"
  }

  if name == "timer_elapsed" {
    return "Float"
  }
  if name == "now_utc" {
    return "List"
  }
  if (((name == "sha1") or (name == "sha256")) or (name == "sha512")) or (name == "sha3_384") {
    return "Bytes"
  }
  if (((((((name == "windows_available") or (name == "windows_close_library")) or (name == "windows_show_window")) or (name == "windows_destroy_window")) or (name == "windows_pump_messages")) or ((name == "windows_set_window_text") or (name == "windows_move_window"))) or ((name == "windows_set_selection") or (name == "windows_replace_selection"))) or (name == "windows_set_text_color") {
    return "Bool"
  }
  if (((((name == "windows_get_procedure") or (name == "windows_selection")) or (name == "windows_run_process")) or (name == "windows_poll_process")) or (name == "windows_wait_process")) or (name == "windows_terminate_process") {
    return "List"
  }
  if ((((name == "windows_next_event") or (name == "windows_find_text")) or (name == "windows_open_file_dialog")) or (name == "windows_save_file_dialog")) or (name == "windows_wait_event") {
    return "VTime"
  }
  if name == "windows_load_icon" {
    return "Int"
  }
  if name == "windows_create_accelerators" {
    return "Int"
  }
  if ((name == "windows_set_window_icon") or (name == "windows_destroy_icon")) or (name == "windows_set_deferred_close") {
    return "Bool"
  }
  if (name == "windows_set_accelerators") or (name == "windows_destroy_accelerators") {
    return "Bool"
  }
  if name == "windows_create_menu" {
    return "Int"
  }
  if (((name == "windows_append_menu_item") or (name == "windows_append_submenu")) or (name == "windows_append_menu_separator")) or (name == "windows_set_menu") {
    return "Bool"
  }
  if ((name == "windows_set_menu_item_enabled") or (name == "windows_set_menu_item_checked")) or (name == "windows_destroy_menu") {
    return "Bool"
  }
  if ((((name == "windows_set_background_color") or (name == "windows_set_text_bold")) or (name == "windows_set_text_font")) or (name == "windows_can_undo")) or (name == "windows_undo") {
    return "Bool"
  }
  if (((name == "windows_can_redo") or (name == "windows_redo")) or (name == "windows_clear_history")) or (name == "windows_scroll_lines") {
    return "Bool"
  }
  if name == "windows_focus" {
    return "Bool"
  }
  if ((name == "windows_event_mask") or (name == "windows_set_event_mask")) or (name == "windows_first_visible_line") {
    return "Int"
  }
  if name == "windows_window_text" {
    return "Str"
  }
  if name == "windows_utf16" {
    return "Bytes"
  }
  if ((((((name == "windows_load_library") or (name == "windows_call")) or (name == "windows_last_error")) or (name == "windows_message_box")) or (name == "windows_create_window")) or (name == "windows_run_message_loop")) or (name == "windows_line_count") {
    return "Int"
  }

  if name == "int" {
    return "Int"
  }

  if name == "float" {
    return "Float"
  }

  if name == "bool" {
    return "Bool"
  }

  if name == "str" {
    return "Str"
  }

  if name == "bytes" {
    return "Bytes"
  }

  if name == "json" {
    return "Json"
  }

  if name == "split" {
    return "List"
  }

  if name == "len" {
    return "Int"
  }

  if name == "contains" {
    return "Bool"
  }

  if name == "join" {
    return "Str"
  }

  if name == "ord" {
    return "Int"
  }

  if name == "char" {
    return "Str"
  }

  if name == "pow" {
    VTime base_type = checker.expr_type(get(args, 0), names)

    if base_type == "Int" {
      return "Int"
    }

    if base_type == "Float" {
      return "Float"
    }
  }

  return "VTime"
}

func checker.expr_type(expression, names) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_INT {
    return "Int"
  }

  if kind == parser.EXPR_FLOAT {
    return "Float"
  }

  if kind == parser.EXPR_STR {
    return "Str"
  }

  if kind == parser.EXPR_BOOL {
    return "Bool"
  }

  if kind == parser.EXPR_NONE {
    return "None"
  }

  if kind == parser.EXPR_INPUT {
    return "Input"
  }

  if kind == parser.EXPR_SECRET_INPUT {
    return "SecretInput"
  }

  if kind == parser.EXPR_VAR {
    VTime entry = checker.find_name(names, parser.expr_value(expression))

    if entry == NONE {
      return "VTime"
    }

    return checker.entry_value_type(entry)
  }

  if kind == parser.EXPR_SELF {
    return "Bool"
  }

  if kind == parser.EXPR_TAG {
    return checker.expr_type(get(expression, 1), names)
  }

  if kind == parser.EXPR_LIST {
    return "List"
  }

  if kind == parser.EXPR_INDEX {
    return "VTime"
  }

  if kind == parser.EXPR_SLICE {
    VTime target_type = checker.expr_type(get(expression, 1), names)

    if target_type == "Str" {
      return "Str"
    }

    if target_type == "Bytes" {
      return "Bytes"
    }

    if target_type == "List" {
      return "List"
    }

    return "VTime"
  }

  if kind == parser.EXPR_UNARY {
    if get(expression, 1) == "not" {
      return "Bool"
    }

    return checker.expr_type(get(expression, 2), names)
  }

  if kind == parser.EXPR_BINARY {
    VTime op = get(expression, 1)

    if op == "==" {
      return "Bool"
    }

    if op == "!=" {
      return "Bool"
    }

    if op == ">" {
      return "Bool"
    }

    if op == "<" {
      return "Bool"
    }

    if op == ">=" {
      return "Bool"
    }

    if op == "<=" {
      return "Bool"
    }

    if op == "and" {
      return "Bool"
    }

    if op == "or" {
      return "Bool"
    }

    VTime left_type = checker.expr_type(get(expression, 2), names)
    VTime right_type = checker.expr_type(get(expression, 3), names)

    if op == "**" {
      if right_type == "Int" {
        if left_type == "Int" {
          return "Int"
        }

        if left_type == "Float" {
          return "Float"
        }
      }

      return "VTime"
    }

    if left_type == right_type {
      if left_type == "Int" {
        return "Int"
      }

      if left_type == "Float" {
        return "Float"
      }
    }

    return "VTime"
  }

  if kind == parser.EXPR_CALL {
    return checker.call_result_type(get(expression, 1), get(expression, 2), names)
  }

  return "VTime"
}

func checker.expr_int_literal(expression) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_INT {
    return get(expression, 1)
  }

  if (kind == parser.EXPR_UNARY) and (get(expression, 1) == "-") {
    VTime inner = checker.expr_int_literal(get(expression, 2))
    if inner != NONE {
      return 0 - inner
    }
  }

  return NONE
}

func checker.match_number(expression) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_INT {
    return ["Int", get(expression, 1)]
  }

  if kind == parser.EXPR_FLOAT {
    return ["Float", get(expression, 1)]
  }

  return []
}

func checker.match_number_in_range(value, start, end) {
  if len(value) == 0 {
    return false
  }
  if len(start) == 0 {
    return false
  }
  if len(end) == 0 {
    return false
  }
  if get(value, 0) != get(start, 0) {
    return false
  }
  if get(value, 0) != get(end, 0) {
    return false
  }
  return (get(start, 1) <= get(value, 1)) and (get(value, 1) <= get(end, 1))
}

func checker.match_range_is_empty(pattern) {
  VTime start = checker.match_number(get(pattern, 1))
  VTime end = checker.match_number(get(pattern, 2))

  if len(start) == 0 {
    return false
  }
  if len(end) == 0 {
    return false
  }
  if get(start, 0) != get(end, 0) {
    return false
  }
  return get(start, 1) > get(end, 1)
}

func checker.match_patterns_overlap(left, right) {
  if left == right {
    return true
  }

  VTime left_kind = get(left, 0)
  VTime right_kind = get(right, 0)

  if (left_kind == parser.MATCH_VALUE) and (right_kind == parser.MATCH_VALUE) {
    return get(left, 1) == get(right, 1)
  }

  if (left_kind == parser.MATCH_VALUE) and (right_kind == parser.MATCH_RANGE) {
    return checker.match_number_in_range(checker.match_number(get(left, 1)), checker.match_number(get(right, 1)), checker.match_number(get(right, 2)))
  }

  if (left_kind == parser.MATCH_RANGE) and (right_kind == parser.MATCH_VALUE) {
    return checker.match_number_in_range(checker.match_number(get(right, 1)), checker.match_number(get(left, 1)), checker.match_number(get(left, 2)))
  }

  if (left_kind == parser.MATCH_RANGE) and (right_kind == parser.MATCH_RANGE) {
    VTime left_start = checker.match_number(get(left, 1))
    VTime left_end = checker.match_number(get(left, 2))
    VTime right_start = checker.match_number(get(right, 1))
    VTime right_end = checker.match_number(get(right, 2))

    if len(left_start) == 0 {
      return false
    }
    if len(left_end) == 0 {
      return false
    }
    if len(right_start) == 0 {
      return false
    }
    if len(right_end) == 0 {
      return false
    }
    if get(left_start, 0) != get(left_end, 0) {
      return false
    }
    if get(left_start, 0) != get(right_start, 0) {
      return false
    }
    if get(left_start, 0) != get(right_end, 0) {
      return false
    }
    return (get(left_start, 1) <= get(right_end, 1)) and (get(right_start, 1) <= get(left_end, 1))
  }

  return false
}

func checker.max_expr_protection(expressions, names) {
  VTime protection = "AV"

  pick(expressions): expression {
    protection = checker.max_protection(protection, checker.expr_protection(expression, names))
  }

  return protection
}

func checker.call_result_protection(name, args, names) {
  name = checker.canonical_qualified_call(name)

  if name == "add" {
    return "AV"
  }

  if name == "get" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "pop" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "len" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "ord" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "char" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "int" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "float" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "bool" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "str" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "bytes" {
    return checker.expr_protection(get(args, 0), names)
  }

  if name == "json" {
    return checker.expr_protection(get(args, 0), names)
  }

  return checker.max_expr_protection(args, names)
}

func checker.expr_protection(expression, names) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_SECRET_INPUT {
    return "ASV"
  }

  if kind == parser.EXPR_VAR {
    VTime entry = checker.find_name(names, parser.expr_value(expression))

    if entry == NONE {
      return "AV"
    }

    return checker.entry_protection(entry)
  }

  if kind == parser.EXPR_TAG {
    return get(expression, 2)
  }

  if kind == parser.EXPR_LIST {
    return checker.max_expr_protection(get(expression, 1), names)
  }

  if kind == parser.EXPR_INDEX {
    return checker.max_protection(checker.expr_protection(get(expression, 1), names), checker.expr_protection(get(expression, 2), names))
  }

  if kind == parser.EXPR_SLICE {
    VTime protection = checker.expr_protection(get(expression, 1), names)
    VTime start = get(expression, 2)
    VTime end = get(expression, 3)
    VTime step = get(expression, 4)

    if start != NONE {
      protection = checker.max_protection(protection, checker.expr_protection(start, names))
    }

    if end != NONE {
      protection = checker.max_protection(protection, checker.expr_protection(end, names))
    }

    if step != NONE {
      protection = checker.max_protection(protection, checker.expr_protection(step, names))
    }

    return protection
  }

  if kind == parser.EXPR_UNARY {
    return checker.expr_protection(get(expression, 2), names)
  }

  if kind == parser.EXPR_BINARY {
    return checker.max_protection(checker.expr_protection(get(expression, 2), names), checker.expr_protection(get(expression, 3), names))
  }

  if kind == parser.EXPR_CALL {
    return checker.call_result_protection(get(expression, 1), get(expression, 2), names)
  }

  return "AV"
}

func checker.protection_downgrade(names, name, source, target) {
  return checker.fail(names, join(["protection downgrade `", name, "`: source ", source, ", target ", target], ""))
}

func checker.validate_tag_protection(expression, names) {
  VTime source = checker.expr_protection(get(expression, 1), names)
  VTime target = get(expression, 2)

  if checker.protection_rank(source) <= checker.protection_rank(target) {
    return checker.ok(names)
  }

  return checker.protection_downgrade(names, "list element", source, target)
}

func checker.validate_decl_protection(node, names) {
  VTime name = get(node, 2)
  VTime expression = get(node, 3)
  VTime kind = parser.expr_kind(expression)
  VTime target = checker.decl_protection(get(node, 1))

  if kind == parser.EXPR_SECRET_INPUT {
    if target != "ASV" {
      return checker.fail(names, join(["secret expression denied `", name, "`"], ""))
    }
  }

  if target == "SASV" {
    if kind == parser.EXPR_INPUT {
      return checker.fail(names, join(["secret expression denied `", name, "`"], ""))
    }

    if kind == parser.EXPR_SECRET_INPUT {
      return checker.fail(names, join(["secret expression denied `", name, "`"], ""))
    }
  }

  VTime source = checker.expr_protection(expression, names)

  if checker.protection_rank(source) > checker.protection_rank(target) {
    return checker.protection_downgrade(names, name, source, target)
  }

  return checker.ok(names)
}

func checker.validate_assignment_protection(node, names) {
  VTime name = get(node, 1)
  VTime op = get(node, 2)
  VTime entry = checker.find_name(names, name)
  VTime source = checker.expr_protection(get(node, 3), names)

  if checker.entry_role(entry) == "VTime" {
    if op != "=" {
      source = checker.max_protection(checker.entry_protection(entry), source)
    }

    names = checker.update_name_protection(names, name, source)
    VTime alias = NONE
    if op == "=" {
      alias = checker.expr_list_alias(get(node, 3), names)
    }
    return checker.ok(checker.update_name_list_alias(names, name, alias))
  }

  VTime target = checker.entry_protection(entry)

  if checker.protection_rank(source) > checker.protection_rank(target) {
    return checker.protection_downgrade(names, name, source, target)
  }

  return checker.ok(names)
}

func checker.validate_public_expression(expression, names, channel) {
  if expression == NONE {
    return checker.ok(names)
  }

  if checker.expr_protection(expression, names) == "AV" {
    return checker.ok(names)
  }

  return checker.fail(names, join(["secret expression denied `", channel, "`"], ""))
}

func checker.validate_public_expressions(expressions, names, channel) {
  pick(expressions): expression {
    VTime state = checker.validate_public_expression(expression, names, channel)
    if get(state, 0) != checker.STATUS_OK {
      return state
    }
  }
  return checker.ok(names)
}

func checker.type_allows(actual, expected) {
  if actual == "VTime" {
    return true
  }

  return actual == expected
}

func checker.type_is_dynamic(actual) {
  if actual == "VTime" {
    return true
  }

  if actual == "None" {
    return true
  }

  if actual == "Input" {
    return true
  }

  return actual == "SecretInput"
}

func checker.type_is_assignable(actual, expected) {
  if checker.type_is_dynamic(actual) {
    return true
  }

  return actual == expected
}

func checker.type_mismatch(names, name, expected, actual) {
  return checker.fail(names, join(["type mismatch `", name, "`: expected ", expected, ", got ", actual], ""))
}

func checker.validate_decl_type(node, names) {
  VTime name = get(node, 2)
  VTime expected = checker.decl_value_type(get(node, 1))
  VTime actual = checker.expr_type(get(node, 3), names)

  if checker.type_is_assignable(actual, expected) {
    return checker.ok(names)
  }

  return checker.type_mismatch(names, name, expected, actual)
}

func checker.validate_assignment_type(node, names) {
  VTime name = get(node, 1)
  VTime entry = checker.find_name(names, name)

  if checker.entry_role(entry) == "VTime" {
    return checker.ok(names)
  }

  VTime expected = checker.entry_value_type(entry)
  VTime actual = checker.expr_type(get(node, 3), names)

  if checker.type_is_assignable(actual, expected) {
    return checker.ok(names)
  }

  return checker.type_mismatch(names, name, expected, actual)
}

func checker.type_is_pickable(actual) {
  if actual == "VTime" {
    return true
  }

  if actual == "Str" {
    return true
  }

  if actual == "Bytes" {
    return true
  }

  return actual == "List"
}

func checker.type_is_numeric(actual) {
  if actual == "VTime" {
    return true
  }

  if actual == "Int" {
    return true
  }

  return actual == "Float"
}

func checker.binary_is_arithmetic(op) {
  if op == "+" {
    return true
  }

  if op == "-" {
    return true
  }

  if op == "*" {
    return true
  }

  if op == "/" {
    return true
  }

  return op == "%"
}

func checker.binary_is_power(op) {
  return op == "**"
}

func checker.binary_is_equality(op) {
  if op == "==" {
    return true
  }

  return op == "!="
}

func checker.binary_is_ordered(op) {
  if op == ">" {
    return true
  }

  if op == ">=" {
    return true
  }

  if op == "<" {
    return true
  }

  return op == "<="
}

func checker.binary_is_logical(op) {
  if op == "and" {
    return true
  }

  return op == "or"
}

func checker.validate_unary_operand_types(expression, names) {
  VTime op = get(expression, 1)
  VTime actual = checker.expr_type(get(expression, 2), names)

  if op == "-" {
    if checker.type_is_numeric(actual) {
      return checker.ok(names)
    }
  }

  if op == "not" {
    if checker.type_allows(actual, "Bool") {
      return checker.ok(names)
    }
  }

  return checker.fail(names, join(["invalid unary operation for ", actual], ""))
}

func checker.validate_binary_operand_types(expression, names) {
  VTime op = get(expression, 1)
  VTime left = checker.expr_type(get(expression, 2), names)
  VTime right = checker.expr_type(get(expression, 3), names)

  if left == "VTime" {
    return checker.ok(names)
  }

  if right == "VTime" {
    return checker.ok(names)
  }

  if checker.binary_is_arithmetic(op) {
    if left == right {
      if left == "Int" {
        return checker.ok(names)
      }

      if left == "Float" {
        return checker.ok(names)
      }
    }

    return checker.fail(names, join(["arithmetic requires matching Int or Float values, got ", left, " and ", right], ""))
  }

  if checker.binary_is_power(op) {
    if right == "Int" {
      if left == "Int" {
        return checker.ok(names)
      }

      if left == "Float" {
        return checker.ok(names)
      }
    }

    return checker.fail(names, join(["power requires Int/Float base and Int exponent, got ", left, " and ", right], ""))
  }

  if checker.binary_is_equality(op) {
    if left == "None" {
      return checker.ok(names)
    }

    if right == "None" {
      return checker.ok(names)
    }

    if left == right {
      return checker.ok(names)
    }

    return checker.fail(names, join(["comparison requires matching types, got ", left, " and ", right], ""))
  }

  if checker.binary_is_ordered(op) {
    if left == right {
      if left == "Int" {
        return checker.ok(names)
      }

      if left == "Float" {
        return checker.ok(names)
      }
    }

    return checker.fail(names, join(["ordered comparison requires matching Int or Float values, got ", left, " and ", right], ""))
  }

  if checker.binary_is_logical(op) {
    if left == "Bool" {
      if right == "Bool" {
        return checker.ok(names)
      }
    }

    return checker.fail(names, join(["logical operation requires Bool values, got ", left, " and ", right], ""))
  }

  return checker.fail(names, join(["unknown binary operator `", op, "`"], ""))
}

func checker.validate_index_operand_types(expression, names) {
  VTime target = checker.expr_type(get(expression, 1), names)

  if checker.type_is_pickable(target) != true {
    return checker.fail(names, join(["index access requires Str, Bytes, List, or VTime, got ", target], ""))
  }

  VTime index = checker.expr_type(get(expression, 2), names)

  if checker.type_allows(index, "Int") != true {
    return checker.fail(names, join(["index must be Int or VTime, got ", index], ""))
  }

  return checker.ok(names)
}

func checker.validate_slice_target_type(expression, names) {
  VTime target = checker.expr_type(get(expression, 1), names)

  if checker.type_is_pickable(target) {
    return checker.ok(names)
  }

  return checker.fail(names, join(["slice requires Str, Bytes, List, or VTime, got ", target], ""))
}

func checker.validate_slice_bound_type(bound, names) {
  VTime actual = checker.expr_type(bound, names)

  if checker.type_allows(actual, "Int") {
    return checker.ok(names)
  }

  return checker.fail(names, join(["slice bounds must be Int or VTime, got ", actual], ""))
}

func checker.validate_condition_type(expression, names) {
  VTime actual = checker.expr_type(expression, names)

  if checker.type_allows(actual, "Bool") {
    return checker.ok(names)
  }

  return checker.type_mismatch(names, "condition", "Bool", actual)
}

func checker.validate_pick_type(expression, names) {
  VTime actual = checker.expr_type(expression, names)

  if checker.type_is_pickable(actual) {
    return checker.ok(names)
  }

  return checker.fail(names, join(["pick requires Str, Bytes, List, or VTime, got ", actual], ""))
}

func checker.validate_named_list_target(expression, names, operation) {
  if parser.expr_kind(expression) != parser.EXPR_VAR {
    return checker.fail(names, join([operation, " requires a named List or VTime target"], ""))
  }

  VTime entry = checker.find_name(names, parser.expr_value(expression))

  if entry == NONE {
    return checker.ok(names)
  }

  if checker.entry_role(entry) == "List" {
    return checker.ok(names)
  }

  if checker.entry_role(entry) == "VTime" {
    return checker.ok(names)
  }

  return checker.fail(names, join([operation, " requires a named List or VTime target"], ""))
}

func checker.validate_builtin_argument_types(expression, names) {
  VTime name = checker.canonical_qualified_call(get(expression, 1))
  name = checker.memory_runtime_name(name)

  if checker.is_builtin_call(name) != true {
    return checker.ok(names)
  }

  VTime args = get(expression, 2)

  if name == "raw_free" {
    return checker.ok(names)
  }
  if name == "raw_scope_exit" {
    return checker.ok(names)
  }
  if name == "raw_scope_leave" {
    return checker.validate_expr(get(args, 0), names)
  }
  if name == "raw_malloc" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, "memory::malloc")
    }
    return checker.fail(names, "memory::malloc(size) requires Int")
  }
  if name == "raw_load" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Str") {
        VTime abi_type = checker.expr_type(get(args, 3), names)
        if (checker.type_allows(abi_type, "List")) or (abi_type == "None") {
          return checker.validate_public_expressions(args, names, "memory::load")
        }
      }
    }
    return checker.fail(names, "memory::load(address, size, permissions, abi) requires Int, Int, Str, and List/NONE")
  }
  if (name == "raw_mark") or (name == "raw_scope_enter") {
    return checker.validate_public_expression(get(args, 0), names, name)
  }
  if name == "raw_where" {
    if checker.type_allows(checker.expr_type(get(args, 1), names), "Int") {
      return checker.validate_public_expressions(args, names, "memory::where")
    }
    return checker.fail(names, "memory::where(source, selector) requires an Int selector")
  }
  if name == "raw_read" {
    if (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 2), names), "Int")) {
      return checker.validate_public_expressions(args, names, "memory::read")
    }
    return checker.fail(names, "memory::read(pointer, offset, count) requires Int offset and count")
  }
  if name == "raw_write" {
    if (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 2), names), "Bytes")) {
      return checker.validate_public_expressions(args, names, "memory::write")
    }
    return checker.fail(names, "memory::write(pointer, offset, value) requires Int offset and Bytes")
  }
  if name == "raw_call" {
    if checker.type_allows(checker.expr_type(get(args, 1), names), "List") {
      return checker.validate_public_expressions(args, names, "memory::call")
    }
    return checker.fail(names, "memory::call(pointer, arguments) requires a List of arguments")
  }

  if name == "int" {
    return checker.ok(names)
  }

  if name == "float" {
    return checker.ok(names)
  }

  if name == "bool" {
    return checker.ok(names)
  }

  if name == "str" {
    return checker.ok(names)
  }

  if name == "bytes" {
    return checker.ok(names)
  }

  if name == "json" {
    return checker.ok(names)
  }

  if name == "timer_start" {
    return checker.ok(names)
  }
  if name == "timer_stop" {
    return checker.ok(names)
  }
  if name == "timer_elapsed" {
    return checker.ok(names)
  }
  if name == "now_utc" {
    return checker.ok(names)
  }
  if (((name == "sha1") or (name == "sha256")) or (name == "sha512")) or (name == "sha3_384") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Bytes") {
      return checker.ok(names)
    }
    return checker.fail(names, join([name, "(value) requires Bytes"], ""))
  }
  if (((((name == "windows_available") or (name == "windows_last_error")) or (name == "windows_run_message_loop")) or (name == "windows_pump_messages")) or (name == "windows_next_event")) or (name == "windows_wait_event") {
    return checker.ok(names)
  }
  if name == "windows_load_library" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Str") {
      return checker.validate_public_expression(get(args, 0), names, "windows::load_library")
    }
    return checker.fail(names, "windows::load_library(name) requires Str")
  }
  if name == "windows_get_procedure" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      if (checker.type_allows(checker.expr_type(get(args, 2), names), "List")) and (checker.type_allows(checker.expr_type(get(args, 3), names), "Str")) {
        return checker.validate_public_expressions(args, names, "windows::get_procedure")
      }
    }
    return checker.fail(names, "windows::get_procedure(library, name, argument_types, result_type) requires Int, Str, List, and Str")
  }
  if name == "windows_call" {
    if checker.type_allows(checker.expr_type(get(args, 1), names), "List") {
      return checker.validate_public_expressions(args, names, "windows::call")
    }
    return checker.fail(names, "windows::call(procedure, arguments) requires a List of arguments")
  }
  if name == "windows_close_library" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, "windows::close_library")
    }
    return checker.fail(names, "windows::close_library(library) requires Int")
  }
  if name == "windows_utf16" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Str") {
      return checker.validate_public_expression(get(args, 0), names, "windows::utf16")
    }
    return checker.fail(names, "windows::utf16(value) requires Str")
  }
  if name == "windows_message_box" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Str")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Int") {
        return checker.validate_public_expressions(args, names, "windows::message_box")
      }
    }
    return checker.fail(names, "windows::message_box(text, caption, kind) requires Str, Str, and Int")
  }
  if name == "windows_create_window" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Str")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Int") {
        return checker.validate_public_expressions(args, names, "windows::create_window")
      }
    }
    return checker.fail(names, "windows::create_window(title, width, height) requires Str, Int, and Int")
  }
  if (name == "windows_show_window") or (name == "windows_destroy_window") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, name)
    }
    return checker.fail(names, join([name, "(window) requires Int"], ""))
  }
  if name == "windows_create_control" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Str") {
        VTime index = 3
        while (index < 10) (-1) {
          if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
            return checker.fail(names, "windows::create_control expects Int geometry, id, and style values")
          }
          index += 1
        }
        return checker.validate_public_expressions(args, names, "windows::create_control")
      }
    }
    return checker.fail(names, "windows::create_control(parent, class, text, id, x, y, width, height, style, extended_style) has invalid argument types")
  }
  if name == "windows_set_window_text" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      return checker.validate_public_expressions(args, names, "windows::set_window_text")
    }
    return checker.fail(names, "windows::set_window_text(window, text) requires Int and Str")
  }
  if name == "windows_window_text" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, "windows::window_text")
    }
    return checker.fail(names, "windows::window_text(window) requires Int")
  }
  if name == "windows_move_window" {
    VTime index = 0
    while (index < 5) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::move_window requires Int window and geometry values")
      }
      index += 1
    }
    if checker.type_allows(checker.expr_type(get(args, 5), names), "Bool") != true {
      return checker.fail(names, "windows::move_window repaint must be Bool")
    }
    return checker.validate_public_expressions(args, names, "windows::move_window")
  }
  if (name == "windows_selection") or (name == "windows_line_count") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, name)
    }
    return checker.fail(names, join([name, "(control) requires Int"], ""))
  }
  if name == "windows_set_selection" {
    VTime index = 0
    while (index < 3) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::set_selection(control, start, end) requires Int values")
      }
      index += 1
    }
    return checker.validate_public_expressions(args, names, "windows::set_selection")
  }
  if name == "windows_replace_selection" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Bool") {
        return checker.validate_public_expressions(args, names, "windows::replace_selection")
      }
    }
    return checker.fail(names, "windows::replace_selection(control, text, can_undo) requires Int, Str, and Bool")
  }
  if name == "windows_set_text_color" {
    VTime index = 0
    while (index < 4) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::set_text_color(control, start, end, color) requires Int values")
      }
      index += 1
    }
    return checker.validate_public_expressions(args, names, "windows::set_text_color")
  }
  if name == "windows_set_background_color" {
    VTime index = 0
    while (index < 4) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::set_background_color(control, start, end, color) requires Int values")
      }
      index += 1
    }
    return checker.validate_public_expressions(args, names, "windows::set_background_color")
  }
  if name == "windows_set_text_bold" {
    VTime index = 0
    while (index < 3) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::set_text_bold(control, start, end, enabled) requires Int range values")
      }
      index += 1
    }
    if checker.type_allows(checker.expr_type(get(args, 3), names), "Bool") != true {
      return checker.fail(names, "windows::set_text_bold enabled must be Bool")
    }
    return checker.validate_public_expressions(args, names, "windows::set_text_bold")
  }
  if name == "windows_set_text_font" {
    VTime index = 0
    while (index < 3) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Int") != true {
        return checker.fail(names, "windows::set_text_font(control, start, end, face, points) requires Int control and range values")
      }
      index += 1
    }
    if checker.type_allows(checker.expr_type(get(args, 3), names), "Str") != true {
      return checker.fail(names, "windows::set_text_font face must be Str")
    }
    if checker.type_allows(checker.expr_type(get(args, 4), names), "Int") != true {
      return checker.fail(names, "windows::set_text_font points must be Int")
    }
    return checker.validate_public_expressions(args, names, "windows::set_text_font")
  }
  if name == "windows_run_process" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Str") != true {
      return checker.fail(names, "windows::run_process program must be Str")
    }
    if checker.type_allows(checker.expr_type(get(args, 1), names), "List") != true {
      return checker.fail(names, "windows::run_process arguments must be List")
    }
    if checker.type_allows(checker.expr_type(get(args, 2), names), "Str") != true {
      return checker.fail(names, "windows::run_process input must be Str")
    }
    if checker.type_allows(checker.expr_type(get(args, 3), names), "Str") != true {
      return checker.fail(names, "windows::run_process working_directory must be Str")
    }
    return checker.validate_public_expressions(args, names, "windows::run_process")
  }
  if name == "windows_start_process" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Str") != true {
      return checker.fail(names, "windows::start_process program must be Str")
    }
    if checker.type_allows(checker.expr_type(get(args, 1), names), "List") != true {
      return checker.fail(names, "windows::start_process arguments must be List")
    }
    if checker.type_allows(checker.expr_type(get(args, 2), names), "Str") != true {
      return checker.fail(names, "windows::start_process input must be Str")
    }
    if checker.type_allows(checker.expr_type(get(args, 3), names), "Str") != true {
      return checker.fail(names, "windows::start_process working_directory must be Str")
    }
    return checker.validate_public_expressions(args, names, "windows::start_process")
  }
  if ((name == "windows_poll_process") or (name == "windows_wait_process")) or (name == "windows_terminate_process") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") != true {
      return checker.fail(names, join([name, " process must be Int"], ""))
    }
    return checker.validate_public_expressions(args, names, name)
  }
  if (name == "windows_open_file_dialog") or (name == "windows_save_file_dialog") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") != true {
      return checker.fail(names, join([name, " owner must be Int"], ""))
    }
    VTime index = 1
    while (index < 7) (-1) {
      if checker.type_allows(checker.expr_type(get(args, index), names), "Str") != true {
        return checker.fail(names, join([name, " dialog options must be Str"], ""))
      }
      index += 1
    }
    return checker.validate_public_expressions(args, names, name)
  }
  if name == "windows_create_menu" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Bool") {
      return checker.validate_public_expression(get(args, 0), names, "windows::create_menu")
    }
    return checker.fail(names, "windows::create_menu(popup) requires Bool")
  }
  if (name == "windows_append_menu_separator") or (name == "windows_destroy_menu") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, name)
    }
    return checker.fail(names, join([name, "(menu) requires Int"], ""))
  }
  if name == "windows_set_menu" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      return checker.validate_public_expressions(args, names, "windows::set_menu")
    }
    return checker.fail(names, "windows::set_menu(window, menu) requires Int and Int")
  }
  if (name == "windows_append_menu_item") or (name == "windows_append_submenu") {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Str") {
        return checker.validate_public_expressions(args, names, name)
      }
    }
    return checker.fail(names, join([name, " requires Int, Int, and Str"], ""))
  }
  if (name == "windows_set_menu_item_enabled") or (name == "windows_set_menu_item_checked") {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      if checker.type_allows(checker.expr_type(get(args, 2), names), "Bool") {
        return checker.validate_public_expressions(args, names, name)
      }
    }
    return checker.fail(names, join([name, " requires Int, Int, and Bool"], ""))
  }
  if name == "windows_load_icon" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Str") != true {
      return checker.fail(names, "windows::load_icon path must be Str")
    }
    if (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 2), names), "Int")) {
      return checker.validate_public_expressions(args, names, "windows::load_icon")
    }
    return checker.fail(names, "windows::load_icon(path, width, height) requires Str, Int, and Int")
  }
  if name == "windows_set_window_icon" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      return checker.validate_public_expressions(args, names, "windows::set_window_icon")
    }
    return checker.fail(names, "windows::set_window_icon(window, icon) requires Int and Int")
  }
  if name == "windows_set_deferred_close" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Bool")) {
      return checker.validate_public_expressions(args, names, "windows::set_deferred_close")
    }
    return checker.fail(names, "windows::set_deferred_close(window, enabled) requires Int and Bool")
  }
  if name == "windows_destroy_icon" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, "windows::destroy_icon")
    }
    return checker.fail(names, "windows::destroy_icon(icon) requires Int")
  }
  if name == "windows_create_accelerators" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "List") {
      return checker.validate_public_expression(get(args, 0), names, "windows::create_accelerators")
    }
    return checker.fail(names, "windows::create_accelerators(entries) requires List")
  }
  if name == "windows_set_accelerators" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      return checker.validate_public_expressions(args, names, "windows::set_accelerators")
    }
    return checker.fail(names, "windows::set_accelerators(window, table) requires Int and Int")
  }
  if name == "windows_destroy_accelerators" {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, "windows::destroy_accelerators")
    }
    return checker.fail(names, "windows::destroy_accelerators(table) requires Int")
  }
  if (((name == "windows_can_undo") or (name == "windows_undo")) or (name == "windows_can_redo")) or (name == "windows_redo") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, name)
    }
    return checker.fail(names, join([name, "(control) requires Int"], ""))
  }
  if (((name == "windows_clear_history") or (name == "windows_event_mask")) or (name == "windows_first_visible_line")) or (name == "windows_focus") {
    if checker.type_allows(checker.expr_type(get(args, 0), names), "Int") {
      return checker.validate_public_expression(get(args, 0), names, name)
    }
    return checker.fail(names, join([name, "(control) requires Int"], ""))
  }
  if (name == "windows_set_event_mask") or (name == "windows_scroll_lines") {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Int")) {
      return checker.validate_public_expressions(args, names, name)
    }
    return checker.fail(names, join([name, " requires Int arguments"], ""))
  }
  if name == "windows_find_text" {
    if (checker.type_allows(checker.expr_type(get(args, 0), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 1), names), "Str")) {
      if (checker.type_allows(checker.expr_type(get(args, 2), names), "Int")) and (checker.type_allows(checker.expr_type(get(args, 3), names), "Int")) {
        if (checker.type_allows(checker.expr_type(get(args, 4), names), "Bool")) and (checker.type_allows(checker.expr_type(get(args, 5), names), "Bool")) {
          return checker.validate_public_expressions(args, names, "windows::find_text")
        }
      }
    }
    return checker.fail(names, "windows::find_text(control, text, start, end, match_case, whole_word) has invalid argument types")
  }

  VTime first_type = checker.expr_type(get(args, 0), names)

  if name == "len" {
    if checker.type_is_pickable(first_type) {
      return checker.ok(names)
    }

    return checker.fail(names, "len(value) requires Str, Bytes, List, or VTime")
  }

  if name == "ord" {
    if checker.type_allows(first_type, "Str") {
      return checker.ok(names)
    }

    return checker.fail(names, "ord(value) requires Str")
  }

  if name == "char" {
    if checker.type_allows(first_type, "Int") {
      return checker.ok(names)
    }

    return checker.fail(names, "char(value) requires Int")
  }

  if name == "pop" {
    if checker.type_allows(first_type, "List") {
      return checker.validate_named_list_target(get(args, 0), names, "pop(list)")
    }

    return checker.fail(names, "pop(list) requires List")
  }

  if name == "add" {
    if checker.type_allows(first_type, "List") {
      return checker.validate_named_list_target(get(args, 0), names, "add(list, value)")
    }

    return checker.fail(names, "add(list, value) requires List")
  }

  VTime second_type = checker.expr_type(get(args, 1), names)

  if name == "set" {
    if (checker.type_allows(first_type, "List")) and (checker.type_allows(second_type, "Int")) {
      return checker.validate_named_list_target(get(args, 0), names, "set(list, index, value)")
    }

    return checker.fail(names, "set(list, index, value) requires List and Int")
  }

  if name == "get" {
    if (checker.type_allows(first_type, "List")) and (checker.type_allows(second_type, "Int")) {
      return checker.ok(names)
    }

    return checker.fail(names, "get(list, index) requires List and Int")
  }

  if name == "split" {
    if (checker.type_allows(first_type, "Str")) and (checker.type_allows(second_type, "Str")) {
      return checker.ok(names)
    }

    return checker.fail(names, "split(value, separator) requires Str and Str")
  }

  if name == "join" {
    if (checker.type_allows(first_type, "List")) and (checker.type_allows(second_type, "Str")) {
      return checker.ok(names)
    }

    return checker.fail(names, "join(values, separator) requires List and Str")
  }

  if name == "contains" {
    if (checker.type_allows(first_type, "Str")) and (checker.type_allows(second_type, "Str")) {
      return checker.ok(names)
    }

    return checker.fail(names, "contains(value, needle) requires Str and Str")
  }

  if name == "pow" {
    if (checker.type_is_numeric(first_type)) and (checker.type_allows(second_type, "Int")) {
      return checker.ok(names)
    }

    return checker.fail(names, "pow(base, exponent) requires Int/Float and Int")
  }

  return checker.ok(names)
}

func checker.validate_call_target(expression, names) {
  VTime name = get(expression, 1)
  VTime args = get(expression, 2)
  VTime resolved = checker.resolve_call_name(names, name)

  if get(resolved, 0) != checker.STATUS_OK {
    return checker.fail(names, get(resolved, 1))
  }

  name = get(resolved, 1)
  set(expression, 1, name)
  VTime availability = checker.require_base_symbol(names, name)

  if get(availability, 0) != checker.STATUS_OK {
    return availability
  }

  VTime builtin_arity = checker.builtin_arity(name)

  if builtin_arity >= 0 {
    if checker.is_memory_builtin(name) {
      if checker.find_function(names, name) == NONE {
        return checker.fail(names, "raw-memory operations require include >> raw_memory")
      }
    }
    if len(args) != builtin_arity {
      return checker.fail(names, join(["wrong argument count `", name, "`"], ""))
    }

    return checker.validate_builtin_argument_types(expression, names)
  }

  VTime entry = checker.find_function(names, name)

  if entry == NONE {
    if checker.find_name(names, name) != NONE {
      return checker.fail(names, join(["invalid function `", name, "`"], ""))
    }

    VTime excluded_library = checker.excluded_library_for_symbol(names, name)

    if excluded_library != NONE {
      return checker.fail(names, join(["excluded symbol `", name, "` from library `", excluded_library, "`"], ""))
    }

    return checker.fail(names, join(["unknown function `", name, "`"], ""))
  }

  if checker.entry_type(entry) != len(args) {
    return checker.fail(names, join(["wrong argument count `", name, "`"], ""))
  }

  if name == "con.print" {
    if checker.max_expr_protection(args, names) != "AV" {
      return checker.fail(names, "secret expression denied `con::print`")
    }
  }

  if name == "con.wait_in" {
    if checker.expr_protection(get(args, 0), names) != "AV" {
      return checker.fail(names, "secret expression denied `con::wait_in`")
    }
  }

  return checker.ok(names)
}

func checker.validate_assignment_target(node, names) {
  VTime name = get(node, 1)
  VTime op = get(node, 2)
  VTime entry = checker.find_name(names, name)

  if entry == NONE {
    return checker.fail(names, join(["unknown assignment target `", name, "`"], ""))
  }

  if checker.entry_role(entry) == "Func" {
    return checker.fail(names, join(["invalid assignment target `", name, "`"], ""))
  }

  if checker.entry_role(entry) == "List" {
    return checker.fail(names, join(["invalid assignment target `", name, "`"], ""))
  }

  if op == "=" {
    return checker.ok(names)
  }

  if checker.entry_role(entry) == "VTime" {
    return checker.ok(names)
  }

  if checker.entry_protection(entry) == "AV" {
    VTime value_type = checker.entry_value_type(entry)

    if value_type == "Int" {
      return checker.ok(names)
    }

    if value_type == "Float" {
      return checker.ok(names)
    }
  }

  return checker.fail(names, join(["invalid compound assignment target `", name, "`"], ""))
}

func checker.validate_secretup_target(node, names) {
  VTime name = get(node, 1)
  VTime entry = checker.find_name(names, name)

  if entry == NONE {
    return checker.fail(names, join(["unknown secretup target `", name, "`"], ""))
  }

  if checker.entry_role(entry) != "Absolute" {
    return checker.fail(names, join(["invalid secretup target `", name, "`"], ""))
  }

  VTime protection = checker.entry_protection(entry)

  if protection == "SASV" {
    return checker.fail(names, join(["invalid secretup target `", name, "`"], ""))
  }

  if protection == "ASV" {
    return checker.ok(checker.update_name_protection(names, name, "SASV"))
  }

  return checker.ok(checker.update_name_protection(names, name, "ASV"))
}

func checker.validate_info_target(names, name) {
  VTime entry = checker.find_name(names, name)

  if entry == NONE {
    return checker.fail(names, join(["unknown info target `", name, "`"], ""))
  }

  if checker.entry_value_type(entry) != "Str" {
    return checker.fail(names, join(["invalid info target `", name, "`"], ""))
  }

  if checker.entry_protection(entry) != "AV" {
    return checker.fail(names, join(["invalid info target `", name, "`"], ""))
  }

  return checker.ok(names)
}

func checker.validate_info_assignment(node, names) {
  VTime type_state = checker.validate_info_target(names, get(node, 1))

  if get(type_state, 0) != checker.STATUS_OK {
    return type_state
  }

  VTime protection_state = checker.validate_info_target(names, get(node, 2))

  if get(protection_state, 0) != checker.STATUS_OK {
    return protection_state
  }

  VTime source = get(node, 3)
  VTime source_entry = checker.find_name(names, source)

  if source_entry == NONE {
    return checker.fail(names, join(["unknown info source `", source, "`"], ""))
  }

  if checker.entry_role(source_entry) != "Absolute" {
    return checker.fail(names, join(["invalid info source `", source, "`"], ""))
  }

  return checker.ok(names)
}

func checker.apply_mutating_call_protection(expression, names) {
  if parser.expr_kind(expression) != parser.EXPR_CALL {
    return checker.ok(names)
  }

  VTime operation = checker.canonical_qualified_call(get(expression, 1))

  if (operation != "add") and (operation != "set") {
    return checker.ok(names)
  }

  VTime args = get(expression, 2)
  VTime target = get(args, 0)

  if parser.expr_kind(target) != parser.EXPR_VAR {
    return checker.ok(names)
  }

  VTime name = parser.expr_value(target)
  VTime entry = checker.find_name(names, name)

  if entry == NONE {
    return checker.ok(names)
  }

  VTime value_index = 1

  if operation == "set" {
    value_index = 2
  }

  VTime protection = checker.max_protection(checker.entry_protection(entry), checker.expr_protection(get(args, value_index), names))
  VTime list_alias = NONE
  if checker.entry_role(entry) == "List" {
    list_alias = name
  } else if checker.entry_role(entry) == "VTime" {
    list_alias = checker.entry_list_alias(entry)
  }

  if list_alias != NONE {
    return checker.ok(checker.raise_list_alias_protection(names, list_alias, protection))
  }

  return checker.ok(checker.update_name_protection(names, name, protection))
}

func checker.validate_expr(expression, names) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_ERROR {
    return checker.fail(names, parser.expr_value(expression))
  }

  if kind == parser.EXPR_INT {
    if parser.expr_value(expression) == NONE {
      return checker.fail(names, "integer literal out of range")
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_FLOAT {
    if parser.expr_value(expression) == NONE {
      return checker.fail(names, "invalid float literal")
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_VAR {
    VTime name = parser.expr_value(expression)
    VTime entry = checker.find_name(names, name)

    if entry != NONE {
      if checker.entry_role(entry) != "Func" {
        return checker.ok(names)
      }
    }

    VTime excluded_library = checker.excluded_library_for_symbol(names, name)

    if excluded_library != NONE {
      return checker.fail(names, join(["excluded symbol `", name, "` from library `", excluded_library, "`"], ""))
    }

    return checker.fail(names, join(["unknown variable `", name, "`"], ""))
  }

  if kind == parser.EXPR_SELF {
    VTime name = parser.expr_value(expression)
    VTime entry = checker.find_name(names, name)

    if entry == NONE {
      return checker.fail(names, join(["unknown variable `", name, "`"], ""))
    }

    if checker.entry_role(entry) != "Absolute" {
      return checker.fail(names, join(["invalid self target `", name, "`"], ""))
    }

    if checker.entry_protection(entry) != "AV" {
      return checker.fail(names, join(["secret expression denied `", name, "`"], ""))
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_UNARY {
    VTime inner_state = checker.validate_expr(get(expression, 2), names)

    if get(inner_state, 0) != checker.STATUS_OK {
      return inner_state
    }

    return checker.validate_unary_operand_types(expression, names)
  }

  if kind == parser.EXPR_BINARY {
    VTime left_state = checker.validate_expr(get(expression, 2), names)

    if get(left_state, 0) != checker.STATUS_OK {
      return left_state
    }

    VTime right_state = checker.validate_expr(get(expression, 3), names)

    if get(right_state, 0) != checker.STATUS_OK {
      return right_state
    }

    return checker.validate_binary_operand_types(expression, names)
  }

  if kind == parser.EXPR_CALL {
    VTime call_state = checker.validate_call_target(expression, names)

    if get(call_state, 0) != checker.STATUS_OK {
      return call_state
    }

    VTime call_name = checker.canonical_qualified_call(get(expression, 1))

    if call_name == "add" {
      return checker.fail(names, "add(list, value) is only valid as a statement")
    }

    if call_name == "set" {
      return checker.fail(names, "set(list, index, value) is only valid as a statement")
    }

    pick(get(expression, 2)): arg {
      VTime arg_state = checker.validate_expr(arg, names)

      if get(arg_state, 0) != checker.STATUS_OK {
        return arg_state
      }
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_LIST {
    pick(get(expression, 1)): item {
      VTime item_state = checker.validate_expr(item, names)

      if get(item_state, 0) != checker.STATUS_OK {
        return item_state
      }
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_INDEX {
    VTime target_state = checker.validate_expr(get(expression, 1), names)

    if get(target_state, 0) != checker.STATUS_OK {
      return target_state
    }

    VTime index_state = checker.validate_expr(get(expression, 2), names)

    if get(index_state, 0) != checker.STATUS_OK {
      return index_state
    }

    return checker.validate_index_operand_types(expression, names)
  }

  if kind == parser.EXPR_SLICE {
    VTime target_state = checker.validate_expr(get(expression, 1), names)

    if get(target_state, 0) != checker.STATUS_OK {
      return target_state
    }

    VTime target_type_state = checker.validate_slice_target_type(expression, names)

    if get(target_type_state, 0) != checker.STATUS_OK {
      return target_type_state
    }

    VTime start = get(expression, 2)
    VTime end = get(expression, 3)
    VTime step = get(expression, 4)

    if start != NONE {
      VTime start_state = checker.validate_expr(start, names)

      if get(start_state, 0) != checker.STATUS_OK {
        return start_state
      }

      VTime start_type_state = checker.validate_slice_bound_type(start, names)

      if get(start_type_state, 0) != checker.STATUS_OK {
        return start_type_state
      }
    }

    if end != NONE {
      VTime end_state = checker.validate_expr(end, names)

      if get(end_state, 0) != checker.STATUS_OK {
        return end_state
      }

      VTime end_type_state = checker.validate_slice_bound_type(end, names)

      if get(end_type_state, 0) != checker.STATUS_OK {
        return end_type_state
      }
    }

    if step != NONE {
      VTime step_state = checker.validate_expr(step, names)

      if get(step_state, 0) != checker.STATUS_OK {
        return step_state
      }

      VTime step_type_state = checker.validate_slice_bound_type(step, names)

      if get(step_type_state, 0) != checker.STATUS_OK {
        return step_type_state
      }
    }

    return checker.ok(names)
  }

  if kind == parser.EXPR_TAG {
    VTime value_state = checker.validate_expr(get(expression, 1), names)

    if get(value_state, 0) != checker.STATUS_OK {
      return value_state
    }

    return checker.validate_tag_protection(expression, names)
  }

  return checker.ok(names)
}

func checker.validate_optional_expr(expression, names) {
  if expression == NONE {
    return checker.ok(names)
  }

  return checker.validate_expr(expression, names)
}

func checker.validate_main_return_types(statements, names) {
  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if kind == parser.NODE_RETURN {
      VTime expression = get(statement, 1)

      if checker.expr_type(expression, names) != "Int" {
        return checker.fail(names, "std::main must return AVInt")
      }

      if checker.expr_protection(expression, names) != "AV" {
        return checker.fail(names, "std::main must return public AVInt")
      }
    }

    if kind == parser.NODE_IF {
      VTime body_state = checker.validate_main_return_types(get(statement, 2), names)

      if get(body_state, 0) != checker.STATUS_OK {
        return body_state
      }

      VTime else_state = checker.validate_main_return_types(get(statement, 3), names)

      if get(else_state, 0) != checker.STATUS_OK {
        return else_state
      }
    }

    if kind == parser.NODE_WHILE {
      VTime loop_state = checker.validate_main_return_types(get(statement, 3), names)

      if get(loop_state, 0) != checker.STATUS_OK {
        return loop_state
      }

      VTime limit_state = checker.validate_main_return_types(get(statement, 4), names)
      if get(limit_state, 0) != checker.STATUS_OK {
        return limit_state
      }

      VTime else_state = checker.validate_main_return_types(get(statement, 5), names)
      if get(else_state, 0) != checker.STATUS_OK {
        return else_state
      }
    }

    if kind == parser.NODE_MATCH {
      pick(get(statement, 2)): case_entry {
        VTime case_state = checker.validate_main_return_types(get(case_entry, 1), names)
        if get(case_state, 0) != checker.STATUS_OK {
          return case_state
        }
      }

      VTime default_state = checker.validate_main_return_types(get(statement, 3), names)
      if get(default_state, 0) != checker.STATUS_OK {
        return default_state
      }
    }

    if kind == parser.NODE_PICK {
      VTime pick_state = checker.validate_main_return_types(get(statement, 3), names)

      if get(pick_state, 0) != checker.STATUS_OK {
        return pick_state
      }
    }
  }

  return checker.ok(names)
}

func checker.validate_statement(node, names, allow_predeclared_func, in_function, loop_depth) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_ERROR {
    return [checker.STATUS_FAIL, names, get(node, 1), NONE]
  }

  if (((kind == parser.NODE_INCLUDE) or (kind == parser.NODE_EXCLUDE)) or (kind == parser.NODE_USING)) or (kind == parser.NODE_NC) {
    return checker.fail(names, "composition directive must be top-level")
  }

  VTime collision_state = checker.validate_decl_collision(node, names, allow_predeclared_func)

  if get(collision_state, 0) != checker.STATUS_OK {
    return collision_state
  }

  if kind == parser.NODE_DECL {
    VTime init_state = checker.validate_expr(get(node, 3), names)

    if get(init_state, 0) != checker.STATUS_OK {
      return init_state
    }

    VTime type_state = checker.validate_decl_type(node, names)

    if get(type_state, 0) != checker.STATUS_OK {
      return type_state
    }

    VTime protection_state = checker.validate_decl_protection(node, names)

    if get(protection_state, 0) != checker.STATUS_OK {
      return protection_state
    }
  }

  if kind == parser.NODE_LIST_DECL {
    VTime list_state = checker.validate_expr(get(node, 2), names)

    if get(list_state, 0) != checker.STATUS_OK {
      return list_state
    }
  }

  if kind == parser.NODE_VTIME_DECL {
    VTime vtime_state = checker.validate_expr(get(node, 2), names)

    if get(vtime_state, 0) != checker.STATUS_OK {
      return vtime_state
    }
  }

  VTime name_state = checker.validate_decl_name(node, names, allow_predeclared_func)
  names = get(name_state, 1)

  if get(name_state, 0) != checker.STATUS_OK {
    return name_state
  }

  if kind == parser.NODE_VTIME_DECL {
    names = checker.update_name_list_alias(names, get(node, 1), checker.expr_list_alias(get(node, 2), names))
  }

  if kind == parser.NODE_ASSIGN {
    VTime target_state = checker.validate_assignment_target(node, names)

    if get(target_state, 0) != checker.STATUS_OK {
      return target_state
    }

    VTime value_state = checker.validate_expr(get(node, 3), names)

    if get(value_state, 0) != checker.STATUS_OK {
      return value_state
    }

    VTime type_state = checker.validate_assignment_type(node, names)

    if get(type_state, 0) != checker.STATUS_OK {
      return type_state
    }

    return checker.validate_assignment_protection(node, names)
  }

  if kind == parser.NODE_SECRETUP {
    return checker.validate_secretup_target(node, names)
  }

  if kind == parser.NODE_INFO_ASSIGN {
    return checker.validate_info_assignment(node, names)
  }

  if kind == parser.NODE_EXPR {
    VTime expression = get(node, 1)

    if parser.expr_kind(expression) == parser.EXPR_CALL {
      VTime call_state = checker.validate_call_target(expression, names)

      if get(call_state, 0) != checker.STATUS_OK {
        return call_state
      }

      VTime statement_call_name = checker.canonical_qualified_call(get(expression, 1))

      if (statement_call_name == "add") or (statement_call_name == "set") {
        pick(get(expression, 2)): arg {
          VTime arg_state = checker.validate_expr(arg, names)

          if get(arg_state, 0) != checker.STATUS_OK {
            return arg_state
          }
        }

        return checker.apply_mutating_call_protection(expression, names)
      }
    }

    VTime expression_state = checker.validate_expr(expression, names)

    if get(expression_state, 0) != checker.STATUS_OK {
      return expression_state
    }

    return checker.ok(names)
  }

  if kind == parser.NODE_OUT {
    VTime expression = get(node, 1)
    VTime expression_state = checker.validate_expr(expression, names)

    if get(expression_state, 0) != checker.STATUS_OK {
      return expression_state
    }

    return checker.validate_public_expression(expression, names, "out")
  }

  if kind == parser.NODE_STOP {
    VTime expression = get(node, 1)
    VTime expression_state = checker.validate_optional_expr(expression, names)

    if get(expression_state, 0) != checker.STATUS_OK {
      return expression_state
    }

    return checker.validate_public_expression(expression, names, "stop")
  }

  if kind == parser.NODE_FAIL {
    VTime expression = get(node, 1)
    VTime expression_state = checker.validate_expr(expression, names)

    if get(expression_state, 0) != checker.STATUS_OK {
      return expression_state
    }

    return checker.validate_public_expression(expression, names, "fail")
  }

  if kind == parser.NODE_ERR {
    if in_function != true {
      return checker.fail(names, "ERR return outside function")
    }

    VTime message = get(node, 1)
    VTime message_state = checker.validate_expr(message, names)
    if get(message_state, 0) != checker.STATUS_OK {
      return message_state
    }
    if checker.type_allows(checker.expr_type(message, names), "Str") != true {
      return checker.type_mismatch(names, "ERR message", "Str", checker.expr_type(message, names))
    }
    VTime public_message = checker.validate_public_expression(message, names, "ERR message")
    if get(public_message, 0) != checker.STATUS_OK {
      return public_message
    }

    VTime code = get(node, 2)
    VTime code_state = checker.validate_expr(code, names)
    if get(code_state, 0) != checker.STATUS_OK {
      return code_state
    }
    if checker.type_allows(checker.expr_type(code, names), "Int") != true {
      return checker.type_mismatch(names, "ERR code", "Int", checker.expr_type(code, names))
    }
    return checker.validate_public_expression(code, names, "ERR code")
  }

  if kind == parser.NODE_BREAK {
    if loop_depth <= 0 {
      return checker.fail(names, "break outside loop")
    }

    return checker.ok(names)
  }

  if kind == parser.NODE_CONTINUE {
    if loop_depth <= 0 {
      return checker.fail(names, "continue outside loop")
    }

    return checker.ok(names)
  }

  if kind == parser.NODE_RETURN {
    if in_function != true {
      return checker.fail(names, "return outside function")
    }

    return checker.validate_expr(get(node, 1), names)
  }

  if kind == parser.NODE_FUNC {
    VTime local_names = names[:]
    VTime params = []

    pick(get(node, 2)): param {
      VTime param_state = checker.validate_name(names, param)

      if get(param_state, 0) != checker.STATUS_OK {
        return param_state
      }

      if checker.value_exists(params, param) {
        return checker.fail(names, join(["duplicate parameter `", param, "`"], ""))
      }

      add(params, param)
      local_names = checker.add_name(local_names, param, "VTime", "VTime")
    }

    VTime body_state = checker.validate_block(get(node, 3), local_names, false, true, 0)

    if get(body_state, 0) != checker.STATUS_OK {
      return body_state
    }

    if checker.is_main_implementation(get(node, 1)) {
      VTime return_state = checker.validate_main_return_types(get(node, 3), get(body_state, 1))

      if get(return_state, 0) != checker.STATUS_OK {
        return return_state
      }
    }

    return checker.ok(checker.merge_existing_names(names, get(body_state, 1)))
  }

  if kind == parser.NODE_IF {
    VTime condition_state = checker.validate_expr(get(node, 1), names)

    if get(condition_state, 0) != checker.STATUS_OK {
      return condition_state
    }

    VTime condition_type_state = checker.validate_condition_type(get(node, 1), names)

    if get(condition_type_state, 0) != checker.STATUS_OK {
      return condition_type_state
    }

    VTime body_state = checker.validate_block(get(node, 2), names[:], false, in_function, loop_depth)

    if get(body_state, 0) != checker.STATUS_OK {
      return body_state
    }

    names = checker.merge_global_names(names, get(body_state, 1))
    VTime else_state = checker.validate_block(get(node, 3), names[:], false, in_function, loop_depth)

    if get(else_state, 0) != checker.STATUS_OK {
      return else_state
    }

    return checker.ok(checker.merge_global_names(names, get(else_state, 1)))
  }

  if kind == parser.NODE_WHILE {
    VTime limit_expression = get(node, 2)
    VTime limit_state = checker.validate_expr(limit_expression, names)

    if get(limit_state, 0) != checker.STATUS_OK {
      return limit_state
    }

    VTime limit_type = checker.expr_type(limit_expression, names)
    if checker.type_allows(limit_type, "Int") != true {
      return checker.type_mismatch(names, "while limit", "Int", limit_type)
    }

    VTime literal_limit = checker.expr_int_literal(limit_expression)
    if literal_limit != NONE {
      if literal_limit < -1 {
        return checker.fail(names, join(["invalid loop limit `", str(literal_limit), "`"], ""))
      }
    }

    VTime condition_state = checker.validate_expr(get(node, 1), names)

    if get(condition_state, 0) != checker.STATUS_OK {
      return condition_state
    }

    VTime condition_type_state = checker.validate_condition_type(get(node, 1), names)

    if get(condition_type_state, 0) != checker.STATUS_OK {
      return condition_type_state
    }

    VTime body_state = checker.validate_block(get(node, 3), names[:], false, in_function, loop_depth + 1)

    if get(body_state, 0) != checker.STATUS_OK {
      return body_state
    }

    names = checker.merge_global_names(names, get(body_state, 1))
    VTime limit_body_state = checker.validate_block(get(node, 4), names[:], false, in_function, loop_depth)
    if get(limit_body_state, 0) != checker.STATUS_OK {
      return limit_body_state
    }
    names = checker.merge_global_names(names, get(limit_body_state, 1))
    VTime else_body_state = checker.validate_block(get(node, 5), names[:], false, in_function, loop_depth)
    if get(else_body_state, 0) != checker.STATUS_OK {
      return else_body_state
    }
    return checker.ok(checker.merge_global_names(names, get(else_body_state, 1)))
  }

  if kind == parser.NODE_MATCH {
    VTime value = get(node, 1)
    VTime value_state = checker.validate_expr(value, names)
    if get(value_state, 0) != checker.STATUS_OK {
      return value_state
    }

    VTime previous_patterns = []
    pick(get(node, 2)): case_entry {
      pick(get(case_entry, 0)): pattern {
        VTime pattern_kind = get(pattern, 0)
        if pattern_kind == parser.MATCH_RANGE {
          if checker.match_range_is_empty(pattern) {
            return checker.fail(names, "invalid match range: start is greater than end")
          }
        }
        pick(previous_patterns): previous_pattern {
          if checker.match_patterns_overlap(previous_pattern, pattern) {
            return checker.fail(names, "overlapping or unreachable match pattern")
          }
        }
        add(previous_patterns, pattern)

        if pattern_kind == parser.MATCH_VALUE {
          VTime comparison_state = checker.validate_expr([parser.EXPR_BINARY, "==", value, get(pattern, 1)], names)
          if get(comparison_state, 0) != checker.STATUS_OK {
            return comparison_state
          }
        } else if pattern_kind == parser.MATCH_RANGE {
          VTime lower_state = checker.validate_expr([parser.EXPR_BINARY, ">=", value, get(pattern, 1)], names)
          if get(lower_state, 0) != checker.STATUS_OK {
            return lower_state
          }
          VTime upper_state = checker.validate_expr([parser.EXPR_BINARY, "<=", value, get(pattern, 2)], names)
          if get(upper_state, 0) != checker.STATUS_OK {
            return upper_state
          }
        } else {
          return checker.fail(names, "invalid match pattern")
        }
      }

      VTime case_state = checker.validate_block(get(case_entry, 1), names[:], false, in_function, loop_depth)
      if get(case_state, 0) != checker.STATUS_OK {
        return case_state
      }
      names = checker.merge_global_names(names, get(case_state, 1))
    }

    VTime default_state = checker.validate_block(get(node, 3), names[:], false, in_function, loop_depth)
    if get(default_state, 0) != checker.STATUS_OK {
      return default_state
    }
    return checker.ok(checker.merge_global_names(names, get(default_state, 1)))
  }

  if kind == parser.NODE_PICK {
    VTime availability = checker.require_base_symbol(names, "pick")

    if get(availability, 0) != checker.STATUS_OK {
      return availability
    }

    VTime value_state = checker.validate_expr(get(node, 1), names)

    if get(value_state, 0) != checker.STATUS_OK {
      return value_state
    }

    VTime value_type_state = checker.validate_pick_type(get(node, 1), names)

    if get(value_type_state, 0) != checker.STATUS_OK {
      return value_type_state
    }

    VTime item_name = get(node, 2)
    VTime item_name_state = checker.validate_name(names, item_name)

    if get(item_name_state, 0) != checker.STATUS_OK {
      return item_name_state
    }

    if checker.name_exists(names, item_name) {
      return checker.fail(names, join(["duplicate name `", item_name, "`"], ""))
    }

    VTime local_names = checker.add_name_with_protection(names[:], item_name, "VTime", "VTime", checker.expr_protection(get(node, 1), names))
    VTime body_state = checker.validate_block(get(node, 3), local_names, false, in_function, loop_depth + 1)

    if get(body_state, 0) != checker.STATUS_OK {
      return body_state
    }

    return checker.ok(checker.merge_global_names(names, get(body_state, 1)))
  }

  return checker.ok(names)
}

func checker.validate_block(statements, names, allow_predeclared_func, in_function, loop_depth) {
  pick(statements): statement {
    VTime state = checker.validate_statement(statement, names, allow_predeclared_func, in_function, loop_depth)
    names = get(state, 1)

    if get(state, 0) != checker.STATUS_OK {
      return checker.attach_failure(state, statement)
    }
  }

  return checker.ok(names)
}

func checker.collect_function_names(statements, names) {
  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_FUNC {
      VTime name = get(statement, 1)
      VTime name_state = checker.validate_name(names, name)

      if get(name_state, 0) != checker.STATUS_OK {
        return checker.attach_failure(name_state, statement)
      }

      if checker.name_exists(names, name) {
        return checker.fail_at_node(names, join(["duplicate name `", name, "`"], ""), statement)
      }

      names = checker.add_name(names, name, "Func", len(get(statement, 2)))
    }
  }

  return checker.ok(names)
}

func checker.without_named_entries(names, removed_names) {
  VTime result = []
  pick(names): entry {
    if checker.value_exists(removed_names, checker.entry_name(entry)) != true {
      add(result, entry)
    }
  }
  return result
}

func checker.add_external_symbols(names, symbols) {
  pick(symbols): symbol {
    if (get(symbol, 1) == "Class") or (get(symbol, 1) == "Struct") {
      continue
    }
    VTime name = get(symbol, 0)
    VTime name_state = checker.validate_name(names, name)

    if get(name_state, 0) != checker.STATUS_OK {
      return name_state
    }

    if checker.name_exists(names, name) {
      return checker.fail(names, join(["duplicate name `", name, "`"], ""))
    }

    if len(symbol) >= 4 {
      names = checker.add_name_with_protection(names, name, get(symbol, 1), get(symbol, 2), get(symbol, 3))
    } else {
      names = checker.add_name(names, name, get(symbol, 1), get(symbol, 2))
    }
  }

  return checker.ok(names)
}

func checker.class_symbols_for_composition(symbols, libraries, includes, exclusions) {
  VTime result = []

  pick(symbols): symbol {
    if get(symbol, 1) == "Class" {
      if checker.exclusion_exists(exclusions, "base", get(symbol, 0)) != true {
        add(result, symbol)
      }
    }
  }

  pick(includes): library_name {
    if (library_name != "base") and (library_name != "std") {
      VTime library = checker.library_entry(libraries, library_name)
      pick(get(library, 1)): symbol {
        if get(symbol, 1) == "Class" {
          if checker.exclusion_exists(exclusions, library_name, get(symbol, 0)) != true {
            add(result, symbol)
          }
        }
      }
    }
  }
  return result
}

func checker.class_surfaces(class_symbols) {
  VTime surfaces = []
  pick(class_symbols): symbol {
    add(surfaces, get(symbol, 2))
  }
  return surfaces
}

func checker.struct_symbols_for_composition(symbols, libraries, includes, exclusions) {
  VTime result = []

  pick(symbols): symbol {
    if get(symbol, 1) == "Struct" {
      if checker.exclusion_exists(exclusions, "base", get(symbol, 0)) != true {
        add(result, symbol)
      }
    }
  }

  pick(includes): library_name {
    if (library_name != "base") and (library_name != "std") {
      VTime library = checker.library_entry(libraries, library_name)
      pick(get(library, 1)): symbol {
        if get(symbol, 1) == "Struct" {
          if checker.exclusion_exists(exclusions, library_name, get(symbol, 0)) != true {
            add(result, symbol)
          }
        }
      }
    }
  }
  return result
}

func checker.struct_surfaces(struct_symbols) {
  VTime surfaces = []
  pick(struct_symbols): symbol {
    add(surfaces, get(symbol, 2))
  }
  return surfaces
}

func checker.export_symbols(names) {
  VTime symbols = []

  pick(names): entry {
    if checker.entry_role(entry) != "VTime" {
      if checker.entry_role(entry) != "Excluded" {
        if checker.entry_role(entry) != "Namespace" {
          if checker.entry_role(entry) != "StrictStd" {
            add(symbols, [checker.entry_name(entry), checker.entry_role(entry), checker.entry_type(entry), checker.entry_protection(entry)])
          }
        }
      }
    }
  }

  return symbols
}

func checker.add_library_symbols(names, library, exclusions) {
  VTime library_name = get(library, 0)
  VTime included_symbols = []

  pick(get(library, 1)): symbol {
    if checker.exclusion_exists(exclusions, library_name, get(symbol, 0)) {
      names = checker.add_name(names, get(symbol, 0), "Excluded", library_name)
    } else {
      add(included_symbols, symbol)
    }
  }

  return checker.add_external_symbols(names, included_symbols)
}

func checker.namespace_symbols(names, namespace_name) {
  VTime symbols = []

  if (namespace_name == "std") or (namespace_name == "base") {
    pick(checker.STANDARD_EXPORTS): symbol {
      if checker.base_symbol_is_excluded(names, symbol) != true {
        add(symbols, symbol)
      }
    }

    return symbols
  }

  VTime prefix = join([namespace_name, "."], "")

  pick(names): entry {
    if checker.entry_role(entry) == "Func" {
      VTime full_name = checker.entry_name(entry)

      if len(full_name) > len(prefix) {
        if full_name[:len(prefix)] == prefix {
          VTime symbol = full_name[len(prefix):]

          if contains(symbol, ".") != true {
            add(symbols, symbol)
          }
        }
      }
    }
  }

  return symbols
}

func checker.add_namespaces(names, namespaces) {
  VTime seen_symbols = []

  pick(namespaces): namespace_name {
    VTime symbols = checker.namespace_symbols(names, namespace_name)

    pick(symbols): symbol {
      if checker.value_exists(seen_symbols, symbol) {
        return checker.fail(names, join(["namespace conflict for `", symbol, "`"], ""))
      }

      add(seen_symbols, symbol)
    }

    add(names, [namespace_name, "Namespace", "Namespace", "AV"])
  }

  return checker.ok(names)
}

func checker.rewrite_expr(expression, names) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_CALL {
    VTime resolved = checker.resolve_call_name(names, get(expression, 1))
    VTime args = []

    pick(get(expression, 2)): arg {
      add(args, checker.rewrite_expr(arg, names))
    }

    return [kind, checker.memory_runtime_name(get(resolved, 1)), args]
  }

  if kind == parser.EXPR_UNARY {
    return [kind, get(expression, 1), checker.rewrite_expr(get(expression, 2), names)]
  }

  if kind == parser.EXPR_BINARY {
    return [kind, get(expression, 1), checker.rewrite_expr(get(expression, 2), names), checker.rewrite_expr(get(expression, 3), names)]
  }

  if kind == parser.EXPR_LIST {
    VTime items = []

    pick(get(expression, 1)): item {
      add(items, checker.rewrite_expr(item, names))
    }

    return [kind, items]
  }

  if kind == parser.EXPR_INDEX {
    return [kind, checker.rewrite_expr(get(expression, 1), names), checker.rewrite_expr(get(expression, 2), names)]
  }

  if kind == parser.EXPR_SLICE {
    VTime start = get(expression, 2)
    VTime finish = get(expression, 3)
    VTime step = get(expression, 4)

    if start != NONE {
      start = checker.rewrite_expr(start, names)
    }

    if finish != NONE {
      finish = checker.rewrite_expr(finish, names)
    }

    if step != NONE {
      step = checker.rewrite_expr(step, names)
    }

    return [kind, checker.rewrite_expr(get(expression, 1), names), start, finish, step]
  }

  if kind == parser.EXPR_TAG {
    return [kind, checker.rewrite_expr(get(expression, 1), names), get(expression, 2)]
  }

  return expression
}

func checker.rewrite_optional_expr(expression, names) {
  if expression == NONE {
    return NONE
  }

  return checker.rewrite_expr(expression, names)
}

func checker.rewrite_node(node, names) {
  VTime kind = parser.node_kind(node)

  if kind == parser.NODE_DECL {
    return [kind, get(node, 1), get(node, 2), checker.rewrite_expr(get(node, 3), names)]
  }

  if (kind == parser.NODE_LIST_DECL) or (kind == parser.NODE_VTIME_DECL) {
    return [kind, get(node, 1), checker.rewrite_expr(get(node, 2), names)]
  }

  if kind == parser.NODE_ASSIGN {
    return [kind, get(node, 1), get(node, 2), checker.rewrite_expr(get(node, 3), names)]
  }

  if kind == parser.NODE_EXPR {
    return [kind, checker.rewrite_expr(get(node, 1), names)]
  }

  if ((kind == parser.NODE_OUT) or (kind == parser.NODE_FAIL)) or (kind == parser.NODE_RETURN) {
    return [kind, checker.rewrite_expr(get(node, 1), names)]
  }

  if kind == parser.NODE_ERR {
    return [kind, checker.rewrite_expr(get(node, 1), names), checker.rewrite_expr(get(node, 2), names)]
  }

  if kind == parser.NODE_STOP {
    return [kind, checker.rewrite_optional_expr(get(node, 1), names)]
  }

  if kind == parser.NODE_FUNC {
    return [kind, get(node, 1), get(node, 2), checker.rewrite_ast(get(node, 3), names)]
  }

  if kind == parser.NODE_IF {
    return [kind, checker.rewrite_expr(get(node, 1), names), checker.rewrite_ast(get(node, 2), names), checker.rewrite_ast(get(node, 3), names)]
  }

  if kind == parser.NODE_WHILE {
    return [kind, checker.rewrite_expr(get(node, 1), names), checker.rewrite_expr(get(node, 2), names), checker.rewrite_ast(get(node, 3), names), checker.rewrite_ast(get(node, 4), names), checker.rewrite_ast(get(node, 5), names)]
  }

  if kind == parser.NODE_MATCH {
    VTime rewritten_cases = []
    pick(get(node, 2)): case_entry {
      VTime rewritten_patterns = []
      pick(get(case_entry, 0)): pattern {
        if get(pattern, 0) == parser.MATCH_VALUE {
          add(rewritten_patterns, [parser.MATCH_VALUE, checker.rewrite_expr(get(pattern, 1), names)])
        } else {
          add(rewritten_patterns, [parser.MATCH_RANGE, checker.rewrite_expr(get(pattern, 1), names), checker.rewrite_expr(get(pattern, 2), names)])
        }
      }
      add(rewritten_cases, [rewritten_patterns, checker.rewrite_ast(get(case_entry, 1), names)])
    }
    return [kind, checker.rewrite_expr(get(node, 1), names), rewritten_cases, checker.rewrite_ast(get(node, 3), names)]
  }

  if kind == parser.NODE_PICK {
    return [kind, checker.rewrite_expr(get(node, 1), names), get(node, 2), checker.rewrite_ast(get(node, 3), names)]
  }

  return node
}

func checker.rewrite_ast(statements, names) {
  VTime rewritten = []

  pick(statements): statement {
    add(rewritten, checker.rewrite_node(statement, names))
  }

  return rewritten
}

func checker.block_definitely_returns(statements) {
  if len(statements) == 0 {
    return false
  }

  VTime last = get(statements, len(statements) - 1)
  VTime kind = parser.node_kind(last)

  if (kind == parser.NODE_RETURN) or (kind == parser.NODE_ERR) {
    return true
  }

  if kind == parser.NODE_IF {
    if len(get(last, 3)) == 0 {
      return false
    }

    return (checker.block_definitely_returns(get(last, 2))) and (checker.block_definitely_returns(get(last, 3)))
  }

  if kind == parser.NODE_MATCH {
    if len(get(last, 3)) == 0 {
      return false
    }

    pick(get(last, 2)): case_entry {
      if checker.block_definitely_returns(get(case_entry, 1)) != true {
        return false
      }
    }

    return checker.block_definitely_returns(get(last, 3))
  }

  return false
}

func checker.validate_executable_shape(statements) {
  VTime main_count = 0

  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if kind == parser.NODE_FUNC {
      if get(statement, 1) == "std::main" {
        main_count += 1

        if len(get(statement, 2)) != 0 {
          return checker.fail_at_node([], "std::main does not accept parameters", statement)
        }

        if checker.block_definitely_returns(get(statement, 3)) != true {
          return checker.fail_at_node([], "std::main may finish without returning AVInt", statement)
        }
      }
    } else if ((kind != parser.NODE_DECL) and (kind != parser.NODE_LIST_DECL)) and (kind != parser.NODE_VTIME_DECL) {
      return checker.fail_at_node([], "executable statement must be inside std::main", statement)
    }
  }

  if main_count == 0 {
    return checker.fail([], "missing AVInt std::main()")
  }

  if main_count > 1 {
    return checker.fail([], "duplicate std::main")
  }

  return checker.ok([])
}

func checker.validate_report_with_library_symbols_mode(statements, symbols, libraries, strict_executable) {
  VTime composition = checker.collect_composition_with_libraries(statements, libraries)

  if get(composition, 0) != checker.STATUS_OK {
    return checker.failure_report(composition)
  }

  VTime checked_statements = get(composition, 1)

  VTime raw_report = rawlower.lower(checked_statements, get(composition, 2))
  if get(raw_report, 0) != true {
    return checker.failure_report(checker.fail_at_node([], get(raw_report, 1), get(raw_report, 2)))
  }
  checked_statements = get(raw_report, 1)

  VTime imported_struct_symbols = checker.struct_symbols_for_composition(symbols, libraries, get(composition, 2), get(composition, 3))
  VTime struct_report = structlower.lower(checked_statements, checker.struct_surfaces(imported_struct_symbols))
  if get(struct_report, 0) != true {
    return checker.failure_report(checker.fail_at_node([], get(struct_report, 1), get(struct_report, 2)))
  }
  checked_statements = get(struct_report, 1)

  VTime imported_class_symbols = checker.class_symbols_for_composition(symbols, libraries, get(composition, 2), get(composition, 3))
  VTime class_report = classlower.lower(checked_statements, get(composition, 2), checker.class_surfaces(imported_class_symbols))
  if get(class_report, 0) != true {
    return checker.failure_report(checker.fail_at_node([], get(class_report, 1), get(class_report, 2)))
  }
  checked_statements = get(class_report, 1)

  if strict_executable {
    VTime shape_state = checker.validate_executable_shape(checked_statements)

    if get(shape_state, 0) != checker.STATUS_OK {
      return checker.failure_report(shape_state)
    }
  }

  VTime names = []

  pick(get(composition, 3)): exclusion {
    if (get(exclusion, 0) == "base") or (get(exclusion, 0) == "std") {
      names = checker.add_name(names, get(exclusion, 1), "Excluded", get(exclusion, 0))
    }
  }

  VTime external_state = checker.add_external_symbols(names, symbols)

  if get(external_state, 0) != checker.STATUS_OK {
    return checker.failure_report(external_state)
  }

  VTime names_with_libraries = get(external_state, 1)

  pick(get(composition, 2)): library_name {
    if (library_name != "base") and (library_name != "std") {
      VTime library_state = checker.add_library_symbols(names_with_libraries, checker.library_entry(libraries, library_name), get(composition, 3))

      if get(library_state, 0) != checker.STATUS_OK {
        return checker.failure_report(library_state)
      }

      names_with_libraries = get(library_state, 1)
    }
  }

  VTime namespace_state = checker.add_namespaces(names_with_libraries, get(composition, 4))

  if get(namespace_state, 0) != checker.STATUS_OK {
    return checker.failure_report(namespace_state)
  }

  names_with_libraries = get(namespace_state, 1)
  names_with_libraries = checker.without_named_entries(names_with_libraries, get(class_report, 3))

  if strict_executable {
    add(names_with_libraries, ["@strict_std", "StrictStd", "Bool", "AV"])
  }

  VTime collect_state = checker.collect_function_names(checked_statements, names_with_libraries)

  if get(collect_state, 0) != checker.STATUS_OK {
    return checker.failure_report(collect_state)
  }

  VTime state = checker.validate_block(checked_statements, get(collect_state, 1), true, false, 0)

  if get(state, 0) != checker.STATUS_OK {
    return checker.failure_report(state)
  }

  VTime exported_symbols = checker.export_symbols(get(state, 1))
  pick(imported_struct_symbols): symbol {
    add(exported_symbols, symbol)
  }
  pick(imported_class_symbols): symbol {
    add(exported_symbols, symbol)
  }

  VTime local_class_symbols = []
  pick(get(struct_report, 2)): surface {
    VTime symbol = [structlower.name(surface), "Struct", surface, "AV"]
    add(exported_symbols, symbol)
    add(local_class_symbols, symbol)
  }
  pick(get(class_report, 2)): surface {
    VTime symbol = [classlower.class_name(surface), "Class", surface, "AV"]
    add(exported_symbols, symbol)
    add(local_class_symbols, symbol)
  }

  return [checker.STATUS_OK, checker.rewrite_ast(checked_statements, get(state, 1)), exported_symbols, get(composition, 2), get(composition, 3), get(composition, 4), local_class_symbols]
}

func checker.validate_report_with_library_symbols(statements, symbols, libraries) {
  return checker.validate_report_with_library_symbols_mode(statements, symbols, libraries, checker.statements_have_main(statements))
}

func checker.validate_executable_report_with_library_symbols(statements, symbols, libraries) {
  return checker.validate_report_with_library_symbols_mode(statements, symbols, libraries, true)
}

func checker.statements_have_main(statements) {
  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_FUNC {
      if get(statement, 1) == "std::main" {
        return true
      }
    }
  }

  return false
}

func checker.validate_report_with_symbols(statements, symbols) {
  return checker.validate_report_with_library_symbols(statements, symbols, [])
}

func checker.validate_report_with_functions(statements, function_names, arities) {
  if len(function_names) != len(arities) {
    return [checker.STATUS_FAIL, "external function table shape mismatch"]
  }

  VTime symbols = []
  VTime index = 0

  while (index < len(function_names)) (-1) {
    add(symbols, [get(function_names, index), "Func", get(arities, index)])
    index += 1
  }

  return checker.validate_report_with_symbols(statements, symbols)
}

func checker.validate_report(statements) {
  return checker.validate_report_with_symbols(statements, [])
}
