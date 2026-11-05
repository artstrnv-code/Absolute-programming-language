# Test VM bootstrap.

AVStr source = "AVInt x = 41 \nx += 1 \nout x"
VTime output = vm.run_source(source)

out "VM output:"
out len(output)
out get(output, 0)
out "Done!"
