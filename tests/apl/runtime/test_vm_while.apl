# Test VM bootstrap with while.

AVStr source = "AVInt x = 0 \nwhile ( x < 3 ) ( 10 ) { \nx += 1 \nout x \n}"
VTime output = vm.run_source(source)

out "VM while output:"
out len(output)
out get(output, 0)
out get(output, 1)
out get(output, 2)
out "Done!"
