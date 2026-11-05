# Test VM bootstrap with if.

AVStr source = "AVInt x = 1 \nif x == 1 { \nx += 41 \nout x \n} \nif x != 42 { \nout 0 \n}"
VTime output = vm.run_source(source)

out "VM if output:"
out len(output)
out get(output, 0)
out "Done!"
