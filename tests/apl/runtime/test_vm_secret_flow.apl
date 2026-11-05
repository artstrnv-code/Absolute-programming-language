AVStr source = "ASVStr token = \"root\" \nVTime tmp = token \nList items = [token] \nout tmp \nout token == token \nout items \nout get(items, 0)"
VTime vm_output = vm.run_source(source)

out "VM secret flow output:"
pick(vm_output): line {
  out line
}
out "Done!"
