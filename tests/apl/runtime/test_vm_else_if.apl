AVStr source = "AVInt x = 2 \nif x == 1 { \nout 1 \n} else if x == 2 { \nout 42 \n} else { \nout 0 \n}"
VTime output = vm.run_source(source)

out "VM else-if output:"
out len(output)
out get(output, 0)
out "Done!"
