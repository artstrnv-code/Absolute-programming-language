AVStr source = "AVInt x = 0 \nwhile ( x < 10 ) ( 20 ) { \nx += 1 \nif x == 2 { \ncontinue \n} \nif x == 5 { \nbreak \n} \nout x \n} \nout x"
VTime vm_output = vm.run_source(source)

out "VM loop flow output:"
pick(vm_output): line {
  out line
}
out "Done!"
