# Test APL checker bootstrap.

AVStr ok_source = "AVInt x = 1 \nList items = [x] \nfunc done() { \nreturn x \n} \nout done()"
AVStr duplicate_var = "AVInt x = 1 \nAVStr x = 2 \nout x"
AVStr duplicate_func = "func go() { \nreturn 1 \n} \nfunc go() { \nreturn 2 \n} \nout go()"
AVStr nested_duplicate = "AVInt x = 1 \nif true { \nAVInt x = 2 \n} \nout x"
AVStr nested_func_duplicate = "func go() { \nreturn 1 \n} \nif true { \nfunc go() { \nreturn 2 \n} \n} \nout go()"

VTime ok_report = bootstrap.compile_report(ok_source)
VTime var_report = bootstrap.compile_report(duplicate_var)
VTime func_report = bootstrap.compile_report(duplicate_func)
VTime nested_report = bootstrap.compile_report(nested_duplicate)
VTime nested_func_report = bootstrap.compile_report(nested_func_duplicate)

out "Checker reports:"
out get(ok_report, 0)
out ir.opcode(get(get(ok_report, 1), 0))
out get(var_report, 0)
out get(var_report, 1)
out get(func_report, 0)
out get(func_report, 1)
out get(nested_report, 0)
out get(nested_report, 1)
out get(nested_func_report, 0)
out get(nested_func_report, 1)
out "Done!"
