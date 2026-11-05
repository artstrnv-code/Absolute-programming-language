# Test VM bootstrap with else.

AVStr source = "AVInt x = 1 \nif x == 2 { \nout 0 \n} else { \nx += 41 \nout x \n}"
VTime output = vm.run_source(source)

out "VM else output:"
out len(output)
out get(output, 0)
out "Done!"
