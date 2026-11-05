AVStr source = "AVBool a = true \nAVBool b = false \nASVBool hidden = true \nif (a == true) and (b != true) { \nout \"ok\" \n} \nif (hidden == true) or (b == true) { \nout hidden == true \n}"
VTime output = vm.run_source(source)

out "VM logic output:"

pick(output): line {
  out line
}

out "Done!"
