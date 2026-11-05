# Test APL checker expression operand and control-value type validation.

List operand_type_sources = [
  "AVInt integer = 1 \nAVFloat decimal = 1.0 \nAVBool flag = true \nAVInt sum = integer + 2 \nAVFloat quotient = decimal / 2.0 \nAVBool equal = integer == 1 \nAVBool ordered = decimal >= 1.0 \nAVBool logical = flag and true \nif flag { \nout integer \n} \nwhile (integer < 2) (1) { \ninteger += 1 \n} \npick([integer]): item { \nout item \n}",
  "VTime value = 1 \nVTime other = value + true \nVTime index = true \nVTime item = value[index] \nVTime part = value[index:index:index] \nif value { \nout other \n} \nwhile (value) (1) { \nvalue = false \n} \npick(value): picked { \nout picked \n}",
  "VTime value = -true",
  "VTime value = not 1",
  "VTime value = 1 + 1.0",
  "VTime value = bytes(1) + bytes(2)",
  "VTime value = 1 == bytes(1)",
  "VTime value = 1 < 1.0",
  "VTime value = true and 1",
  "if 1 { \nout 1 \n}",
  "while (bytes(1)) (1) { \nout 1 \n}",
  "pick(1): item { \nout item \n}",
  "VTime value = 1[0]",
  "VTime value = [1][true]",
  "VTime value = true[:]",
  "VTime value = [1][1.0:]"
]

out "Checker operand type reports:"

pick(operand_type_sources): operand_type_source {
  VTime operand_type_report = bootstrap.compile_report(operand_type_source)
  out get(operand_type_report, 0)

  if get(operand_type_report, 0) == checker.STATUS_FAIL {
    out get(operand_type_report, 1)
  }
}

out "Done!"
