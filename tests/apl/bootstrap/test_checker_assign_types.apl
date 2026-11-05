# Test APL checker declaration and assignment type validation.

List assignment_type_sources = [
  "AVInt integer = 1 \nAVFloat decimal = 1.0 \nAVBool flag = true \nAVStr text = str(1) \nAVBytes raw = bytes(text) \nAVJson document = json(text) \ninteger = 2 \ndecimal = 2.0 \nflag = false \ntext = str(2) \nraw = bytes(text) \ndocument = json(text)",
  "AVInt value = input \nvalue = NONE \nVTime dynamic = 1 \ndynamic = str(dynamic) \nvalue = dynamic",
  "AVInt value = str(1)",
  "AVFloat value = 1",
  "AVBool value = 1",
  "AVStr value = 1",
  "AVBytes value = str(1)",
  "AVJson value = str(1)",
  "AVInt value = 1 \nvalue = str(2)",
  "AVInt value = 1 \nvalue += 1.0"
]

out "Checker assignment type reports:"

pick(assignment_type_sources): assignment_type_source {
  VTime assignment_type_report = bootstrap.compile_report(assignment_type_source)
  out get(assignment_type_report, 0)

  if get(assignment_type_report, 0) == checker.STATUS_FAIL {
    out get(assignment_type_report, 1)
  }
}

out "Done!"
