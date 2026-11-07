# APL loaded-image verifier.
# It treats decoded artifacts as untrusted and validates their complete shape.

func verifier.is_int(value) {
  if value == NONE {
    return false
  }

  return int(str(value)) == value
}

func verifier.is_float(value) {
  if value == NONE {
    return false
  }

  return float(str(value)) == value
}

func verifier.is_str(value) {
  if value == NONE {
    return false
  }

  return str(value) == value
}

func verifier.is_bool(value) {
  if value == true {
    return true
  }

  return value == false
}

func verifier.tag_is_valid(tag) {
  if tag == "AV" {
    return true
  }

  if tag == "ASV" {
    return true
  }

  return tag == "SASV"
}

func verifier.unary_op_is_valid(op) {
  if op == "-" {
    return true
  }

  return op == "not"
}

func verifier.builtin_arity(name) {
  if name == "raw_malloc" {
    return 1
  }
  if name == "raw_load" {
    return 4
  }
  if name == "raw_mark" {
    return 1
  }
  if name == "raw_free" {
    return 0
  }
  if name == "raw_where" {
    return 2
  }
  if name == "raw_read" {
    return 3
  }
  if name == "raw_write" {
    return 3
  }
  if name == "raw_call" {
    return 2
  }
  if name == "raw_scope_enter" {
    return 1
  }
  if name == "raw_scope_exit" {
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

  if name == "sha1" {

    return 1

  }
  if name == "sha256" {
    return 1
  }
  if name == "sha512" {
    return 1
  }
  if name == "sha3_384" {
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
  if name == "windows_create_callback" {
    return 2
  }
  if (name == "windows_next_callback") or (name == "windows_destroy_callback") {
    return 1
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
  if (name == "windows_load_icon") or (name == "windows_load_resource_icon") {
    return 3
  }
  if name == "windows_set_deferred_close" {
    return 2
  }
  if name == "windows_set_timer" {
    return 3
  }
  if ((name == "windows_kill_timer") or (name == "windows_enable_window")) or (name == "windows_invalidate_window") {
    return 2
  }
  if name == "windows_client_size" {
    return 1
  }
  if name == "windows_set_minimum_size" {
    return 3
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

func verifier.exprs_are_valid(expressions, function_count) {
  pick(expressions): expression {
    if verifier.expr_is_valid(expression, function_count) != true {
      return false
    }
  }

  return true
}

func verifier.literal_is_valid(kind, value) {
  if kind == parser.EXPR_INT {
    return verifier.is_int(value)
  }

  if kind == parser.EXPR_FLOAT {
    return verifier.is_float(value)
  }

  if kind == parser.EXPR_STR {
    return verifier.is_str(value)
  }

  if kind == parser.EXPR_BOOL {
    return verifier.is_bool(value)
  }

  return false
}

func verifier.expr_is_valid(expression, function_count) {
  VTime opcode = ir.expr_opcode(expression)

  if opcode == ir.EXPR_LITERAL {
    if len(expression) != 3 {
      return false
    }

    return verifier.literal_is_valid(get(expression, 1), get(expression, 2))
  }

  if opcode == ir.EXPR_LOAD {
    if len(expression) != 3 {
      return false
    }

    if get(expression, 1) != parser.EXPR_VAR {
      return false
    }

    return verifier.is_str(get(expression, 2))
  }

  if opcode == ir.EXPR_SELF {
    if len(expression) != 3 {
      return false
    }

    if get(expression, 1) != parser.EXPR_SELF {
      return false
    }

    return verifier.is_str(get(expression, 2))
  }

  if opcode == ir.EXPR_NONE {
    if len(expression) != 3 {
      return false
    }

    if get(expression, 1) != parser.EXPR_NONE {
      return false
    }

    return get(expression, 2) == NONE
  }

  if opcode == ir.EXPR_INPUT {
    if len(expression) != 3 {
      return false
    }

    if get(expression, 1) != parser.EXPR_INPUT {
      return false
    }

    return get(expression, 2) == NONE
  }

  if opcode == ir.EXPR_SECRET_INPUT {
    if len(expression) != 3 {
      return false
    }

    if get(expression, 1) != parser.EXPR_SECRET_INPUT {
      return false
    }

    return get(expression, 2) == NONE
  }

  if opcode == ir.EXPR_UNARY {
    if len(expression) != 3 {
      return false
    }

    if verifier.unary_op_is_valid(get(expression, 1)) != true {
      return false
    }

    return verifier.expr_is_valid(get(expression, 2), function_count)
  }

  if opcode == ir.EXPR_BINARY {
    if len(expression) != 4 {
      return false
    }

    if parser.is_binary_op(get(expression, 1)) != true {
      return false
    }

    if verifier.expr_is_valid(get(expression, 2), function_count) != true {
      return false
    }

    return verifier.expr_is_valid(get(expression, 3), function_count)
  }

  if opcode == ir.EXPR_CALL {
    if len(expression) != 3 {
      return false
    }

    VTime expected = verifier.builtin_arity(get(expression, 1))

    if expected < 0 {
      return false
    }

    if len(get(expression, 2)) != expected {
      return false
    }

    return verifier.exprs_are_valid(get(expression, 2), function_count)
  }

  if opcode == ir.EXPR_CALL_SLOT {
    if len(expression) != 3 {
      return false
    }

    VTime function_index = get(expression, 1)

    if verifier.is_int(function_index) != true {
      return false
    }

    if function_index < 0 {
      return false
    }

    if function_index >= function_count {
      return false
    }

    return verifier.exprs_are_valid(get(expression, 2), function_count)
  }

  if opcode == ir.EXPR_LIST {
    if len(expression) != 2 {
      return false
    }

    return verifier.exprs_are_valid(get(expression, 1), function_count)
  }

  if opcode == ir.EXPR_INDEX {
    if len(expression) != 3 {
      return false
    }

    if verifier.expr_is_valid(get(expression, 1), function_count) != true {
      return false
    }

    return verifier.expr_is_valid(get(expression, 2), function_count)
  }

  if opcode == ir.EXPR_SLICE {
    if len(expression) != 5 {
      return false
    }

    if verifier.expr_is_valid(get(expression, 1), function_count) != true {
      return false
    }

    if verifier.expr_is_valid(get(expression, 2), function_count) != true {
      return false
    }

    if verifier.expr_is_valid(get(expression, 3), function_count) != true {
      return false
    }

    return verifier.expr_is_valid(get(expression, 4), function_count)
  }

  if opcode == ir.EXPR_TAG {
    if len(expression) != 3 {
      return false
    }

    if verifier.expr_is_valid(get(expression, 1), function_count) != true {
      return false
    }

    return verifier.tag_is_valid(get(expression, 2))
  }

  return false
}

func verifier.instruction_is_valid(instruction, function_count) {
  VTime opcode = ir.opcode(instruction)

  if opcode == ir.OP_DECL {
    if len(instruction) != 4 {
      return false
    }

    if parser.is_decl_keyword(get(instruction, 1)) != true {
      return false
    }

    if verifier.is_str(get(instruction, 2)) != true {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 3), function_count)
  }

  if opcode == ir.OP_LIST_DECL {
    if len(instruction) != 3 {
      return false
    }

    if verifier.is_str(get(instruction, 1)) != true {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 2), function_count)
  }

  if opcode == ir.OP_VTIME_DECL {
    if len(instruction) != 3 {
      return false
    }

    if verifier.is_str(get(instruction, 1)) != true {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 2), function_count)
  }

  if opcode == ir.OP_INFO_ASSIGN {
    if len(instruction) != 4 {
      return false
    }

    if verifier.is_str(get(instruction, 1)) != true {
      return false
    }

    if verifier.is_str(get(instruction, 2)) != true {
      return false
    }

    return verifier.is_str(get(instruction, 3))
  }

  if opcode == ir.OP_ASSIGN {
    if len(instruction) != 4 {
      return false
    }

    if verifier.is_str(get(instruction, 1)) != true {
      return false
    }

    if parser.is_assign_op(get(instruction, 2)) != true {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 3), function_count)
  }

  if opcode == ir.OP_EXPR {
    if len(instruction) != 2 {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 1), function_count)
  }

  if opcode == ir.OP_OUT {
    if len(instruction) != 2 {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 1), function_count)
  }

  if opcode == ir.OP_STOP {
    if len(instruction) != 2 {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 1), function_count)
  }

  if opcode == ir.OP_FAIL {
    if len(instruction) != 2 {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 1), function_count)
  }

  if opcode == ir.OP_ERR_RETURN {
    if len(instruction) != 3 {
      return false
    }
    if verifier.expr_is_valid(get(instruction, 1), function_count) != true {
      return false
    }
    return verifier.expr_is_valid(get(instruction, 2), function_count)
  }

  if opcode == ir.OP_RETURN {
    if len(instruction) != 2 {
      return false
    }

    return verifier.expr_is_valid(get(instruction, 1), function_count)
  }

  if opcode == ir.OP_SECRETUP {
    if len(instruction) != 2 {
      return false
    }

    return verifier.is_str(get(instruction, 1))
  }

  if opcode == ir.OP_BREAK {
    return len(instruction) == 1
  }

  if opcode == ir.OP_CONTINUE {
    return len(instruction) == 1
  }

  if opcode == ir.OP_IF {
    if len(instruction) != 4 {
      return false
    }

    if verifier.expr_is_valid(get(instruction, 1), function_count) != true {
      return false
    }

    if verifier.program_is_valid(get(instruction, 2), function_count) != true {
      return false
    }

    return verifier.program_is_valid(get(instruction, 3), function_count)
  }

  if opcode == ir.OP_WHILE {
    if len(instruction) != 6 {
      return false
    }

    if verifier.expr_is_valid(get(instruction, 1), function_count) != true {
      return false
    }

    if verifier.expr_is_valid(get(instruction, 2), function_count) != true {
      return false
    }

    if verifier.program_is_valid(get(instruction, 3), function_count) != true {
      return false
    }
    if verifier.program_is_valid(get(instruction, 4), function_count) != true {
      return false
    }
    return verifier.program_is_valid(get(instruction, 5), function_count)
  }

  if opcode == ir.OP_MATCH {
    if len(instruction) != 4 {
      return false
    }
    if verifier.expr_is_valid(get(instruction, 1), function_count) != true {
      return false
    }
    pick(get(instruction, 2)): case_entry {
      if len(case_entry) != 2 {
        return false
      }
      pick(get(case_entry, 0)): pattern {
        if get(pattern, 0) == parser.MATCH_VALUE {
          if (len(pattern) != 2) or (verifier.expr_is_valid(get(pattern, 1), function_count) != true) {
            return false
          }
        } else if get(pattern, 0) == parser.MATCH_RANGE {
          if len(pattern) != 3 {
            return false
          }
          if (verifier.expr_is_valid(get(pattern, 1), function_count) != true) or (verifier.expr_is_valid(get(pattern, 2), function_count) != true) {
            return false
          }
        } else {
          return false
        }
      }
      if verifier.program_is_valid(get(case_entry, 1), function_count) != true {
        return false
      }
    }
    return verifier.program_is_valid(get(instruction, 3), function_count)
  }

  if opcode == ir.OP_PICK {
    if len(instruction) != 4 {
      return false
    }

    if verifier.expr_is_valid(get(instruction, 1), function_count) != true {
      return false
    }

    if verifier.is_str(get(instruction, 2)) != true {
      return false
    }

    return verifier.program_is_valid(get(instruction, 3), function_count)
  }

  return false
}

func verifier.program_is_valid(program, function_count) {
  pick(program): instruction {
    if verifier.instruction_is_valid(instruction, function_count) != true {
      return false
    }
  }

  return true
}

func verifier.exprs_contain_call_slot(expressions) {
  pick(expressions): expression {
    if verifier.expr_contains_call_slot(expression) {
      return true
    }
  }

  return false
}

func verifier.expr_contains_call_slot(expression) {
  VTime opcode = ir.expr_opcode(expression)

  if opcode == ir.EXPR_CALL_SLOT {
    return true
  }

  if opcode == ir.EXPR_UNARY {
    return verifier.expr_contains_call_slot(get(expression, 2))
  }

  if opcode == ir.EXPR_BINARY {
    if verifier.expr_contains_call_slot(get(expression, 2)) {
      return true
    }

    return verifier.expr_contains_call_slot(get(expression, 3))
  }

  if opcode == ir.EXPR_CALL {
    return verifier.exprs_contain_call_slot(get(expression, 2))
  }

  if opcode == ir.EXPR_LIST {
    return verifier.exprs_contain_call_slot(get(expression, 1))
  }

  if opcode == ir.EXPR_INDEX {
    if verifier.expr_contains_call_slot(get(expression, 1)) {
      return true
    }

    return verifier.expr_contains_call_slot(get(expression, 2))
  }

  if opcode == ir.EXPR_SLICE {
    if verifier.expr_contains_call_slot(get(expression, 1)) {
      return true
    }

    if verifier.expr_contains_call_slot(get(expression, 2)) {
      return true
    }

    if verifier.expr_contains_call_slot(get(expression, 3)) {
      return true
    }

    return verifier.expr_contains_call_slot(get(expression, 4))
  }

  if opcode == ir.EXPR_TAG {
    return verifier.expr_contains_call_slot(get(expression, 1))
  }

  return false
}

func verifier.instruction_contains_call_slot(instruction) {
  VTime opcode = ir.opcode(instruction)

  if opcode == ir.OP_DECL {
    return verifier.expr_contains_call_slot(get(instruction, 3))
  }

  if opcode == ir.OP_LIST_DECL {
    return verifier.expr_contains_call_slot(get(instruction, 2))
  }

  if opcode == ir.OP_VTIME_DECL {
    return verifier.expr_contains_call_slot(get(instruction, 2))
  }

  if opcode == ir.OP_ASSIGN {
    return verifier.expr_contains_call_slot(get(instruction, 3))
  }

  if opcode == ir.OP_EXPR {
    return verifier.expr_contains_call_slot(get(instruction, 1))
  }

  if opcode == ir.OP_OUT {
    return verifier.expr_contains_call_slot(get(instruction, 1))
  }

  if opcode == ir.OP_STOP {
    return verifier.expr_contains_call_slot(get(instruction, 1))
  }

  if opcode == ir.OP_FAIL {
    return verifier.expr_contains_call_slot(get(instruction, 1))
  }

  if opcode == ir.OP_ERR_RETURN {
    return (verifier.expr_contains_call_slot(get(instruction, 1))) or (verifier.expr_contains_call_slot(get(instruction, 2)))
  }

  if opcode == ir.OP_RETURN {
    return verifier.expr_contains_call_slot(get(instruction, 1))
  }

  if opcode == ir.OP_IF {
    if verifier.expr_contains_call_slot(get(instruction, 1)) {
      return true
    }

    if verifier.program_contains_call_slot(get(instruction, 2)) {
      return true
    }

    return verifier.program_contains_call_slot(get(instruction, 3))
  }

  if opcode == ir.OP_WHILE {
    if verifier.expr_contains_call_slot(get(instruction, 1)) {
      return true
    }

    if verifier.expr_contains_call_slot(get(instruction, 2)) {
      return true
    }

    if verifier.program_contains_call_slot(get(instruction, 3)) {
      return true
    }
    if verifier.program_contains_call_slot(get(instruction, 4)) {
      return true
    }
    return verifier.program_contains_call_slot(get(instruction, 5))
  }

  if opcode == ir.OP_MATCH {
    if verifier.expr_contains_call_slot(get(instruction, 1)) {
      return true
    }
    pick(get(instruction, 2)): case_entry {
      pick(get(case_entry, 0)): pattern {
        if verifier.expr_contains_call_slot(get(pattern, 1)) {
          return true
        }
        if get(pattern, 0) == parser.MATCH_RANGE {
          if verifier.expr_contains_call_slot(get(pattern, 2)) {
            return true
          }
        }
      }
      if verifier.program_contains_call_slot(get(case_entry, 1)) {
        return true
      }
    }
    return verifier.program_contains_call_slot(get(instruction, 3))
  }

  if opcode == ir.OP_PICK {
    if verifier.expr_contains_call_slot(get(instruction, 1)) {
      return true
    }

    return verifier.program_contains_call_slot(get(instruction, 3))
  }

  if opcode == ir.OP_FUNC {
    return verifier.program_contains_call_slot(get(instruction, 3))
  }

  return false
}

func verifier.program_contains_call_slot(program) {
  pick(program): instruction {
    if verifier.instruction_contains_call_slot(instruction) {
      return true
    }
  }

  return false
}

func verifier.str_list_is_valid(values) {
  pick(values): value {
    if verifier.is_str(value) != true {
      return false
    }
  }

  return true
}

func verifier.values_are_unique(values) {
  VTime index = 0

  while (index < len(values)) (-1) {
    VTime previous = 0

    while (previous < index) (-1) {
      if get(values, previous) == get(values, index) {
        return false
      }

      previous += 1
    }

    index += 1
  }

  return true
}

func verifier.params_are_valid(params_list) {
  pick(params_list): params {
    if verifier.str_list_is_valid(params) != true {
      return false
    }

    if verifier.values_are_unique(params) != true {
      return false
    }
  }

  return true
}

func verifier.functions_are_valid(functions) {
  if len(functions) != 4 {
    return false
  }

  VTime names = get(functions, 0)
  VTime params_list = get(functions, 1)
  VTime bodies = get(functions, 2)
  VTime valid = get(functions, 3)

  if valid != true {
    return false
  }

  if verifier.str_list_is_valid(names) != true {
    return false
  }

  if verifier.values_are_unique(names) != true {
    return false
  }

  if len(params_list) != len(names) {
    return false
  }

  if len(bodies) != len(names) {
    return false
  }

  if verifier.params_are_valid(params_list) != true {
    return false
  }

  pick(bodies): body {
    if verifier.program_is_valid(body, len(names)) != true {
      return false
    }
  }

  return true
}

func verifier.library_manifest_is_valid(manifest) {
  if len(manifest) != 2 {
    return false
  }

  if get(manifest, 0) != linker.MANIFEST_MAGIC {
    return false
  }

  VTime libraries = get(manifest, 1)
  VTime names = []

  pick(libraries): library {
    if len(library) != 2 {
      return false
    }

    VTime name = get(library, 0)
    VTime exclusions = get(library, 1)

    if verifier.is_str(name) != true {
      return false
    }

    if checker.composition_name_is_valid(name) != true {
      return false
    }

    if verifier.str_list_is_valid(exclusions) != true {
      return false
    }

    if verifier.values_are_unique(exclusions) != true {
      return false
    }

    pick(exclusions): exclusion {
      if checker.composition_name_is_valid(exclusion) != true {
        return false
      }

      if name == "base" {
        if checker.base_export_is_known(exclusion) != true {
          return false
        }
      }
    }

    add(names, name)
  }

  if len(names) == 0 {
    return false
  }

  if get(names, 0) != "base" {
    return false
  }

  return verifier.values_are_unique(names)
}

func verifier.loaded_image_is_valid(loaded) {
  if len(loaded) != 3 {
    if len(loaded) != 4 {
      return false
    }

    if verifier.library_manifest_is_valid(get(loaded, 3)) != true {
      return false
    }
  }

  if get(loaded, 0) != linker.IMAGE_MAGIC {
    return false
  }

  VTime functions = get(loaded, 2)

  if verifier.functions_are_valid(functions) != true {
    return false
  }

  return verifier.program_is_valid(get(loaded, 1), len(get(functions, 0)))
}
