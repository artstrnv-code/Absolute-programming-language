AVStr source = "List items = [1] \nadd(items, 2) \nAVInt x = 7 \nVTime bad_add = add(x, 9) \nVTime missing_add = add(missing, 1) \nVTime missing_pop = pop(missing) \nout len(items) \nout pop(items) \nout len(items) \nout bad_add \nout missing_add \nout missing_pop"
VTime output = vm.run_source(source)

out "VM list mutation guards output:"

pick(output): line {
  out line
}

out "Done!"
