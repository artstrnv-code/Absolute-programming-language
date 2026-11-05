AVStr duplicate_var = "AVInt x = 1 \nout x \nAVStr x = \"bad\" \nout x"
AVStr duplicate_func = "func same() { \nreturn 1 \n} \nfunc same() { \nreturn 2 \n} \nout same()"
VTime var_output = vm.run_source(duplicate_var)
VTime func_output = vm.run_source(duplicate_func)

out "VM duplicate names output:"
out len(var_output)
out get(var_output, 0)
out get(var_output, 1)
out len(func_output)
out get(func_output, 0)
out "Done!"
