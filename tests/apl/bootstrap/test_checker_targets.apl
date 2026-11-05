# Test APL checker target validation.

AVStr missing_assign = "missing = 1 \nout missing"
AVStr list_assign = "List items = [] \nitems = 1"
AVStr func_assign = "func go() { \nreturn 1 \n} \ngo = 2"
AVStr secret_compound = "ASVInt hidden = 1 \nhidden += 1"
AVStr string_compound = "AVStr text = \"\" \ntext += 1"
AVStr vtime_compound = "VTime temp = 1 \ntemp += 1 \nout temp"
AVStr missing_secretup = "secretup(missing)"
AVStr list_secretup = "List items = [] \nsecretup(items)"
AVStr missing_info_target = "AVStr level = \"\" \ntyp, level = info(level)"
AVStr bad_info_target = "AVInt typ = 1 \nAVStr level = \"\" \ntyp, level = info(typ)"
AVStr missing_info_source = "AVStr typ = \"\" \nAVStr level = \"\" \ntyp, level = info(missing)"

VTime assign_report = bootstrap.compile_report(missing_assign)
VTime list_assign_report = bootstrap.compile_report(list_assign)
VTime func_assign_report = bootstrap.compile_report(func_assign)
VTime secret_compound_report = bootstrap.compile_report(secret_compound)
VTime string_compound_report = bootstrap.compile_report(string_compound)
VTime vtime_compound_report = bootstrap.run_report(vtime_compound)
VTime secretup_report = bootstrap.compile_report(missing_secretup)
VTime list_report = bootstrap.compile_report(list_secretup)
VTime info_target_report = bootstrap.compile_report(missing_info_target)
VTime bad_info_report = bootstrap.compile_report(bad_info_target)
VTime info_source_report = bootstrap.compile_report(missing_info_source)

out "Checker target reports:"
out get(assign_report, 0)
out get(assign_report, 1)
out get(list_assign_report, 0)
out get(list_assign_report, 1)
out get(func_assign_report, 0)
out get(func_assign_report, 1)
out get(secret_compound_report, 0)
out get(secret_compound_report, 1)
out get(string_compound_report, 0)
out get(string_compound_report, 1)
out get(vtime_compound_report, 0)
out get(get(vtime_compound_report, 1), 0)
out get(secretup_report, 0)
out get(secretup_report, 1)
out get(list_report, 0)
out get(list_report, 1)
out get(info_target_report, 0)
out get(info_target_report, 1)
out get(bad_info_report, 0)
out get(bad_info_report, 1)
out get(info_source_report, 0)
out get(info_source_report, 1)
out "Done!"
