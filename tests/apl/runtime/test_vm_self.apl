AVStr source = "AVInt x = 4 \nout x =self= \nx += 1 \nout x =self= \nASVStr token = \"root\" \nout token =self="
VTime output = vm.run_source(source)

out "VM self output:"

pick(output): line {
  out line
}

out "Done!"
