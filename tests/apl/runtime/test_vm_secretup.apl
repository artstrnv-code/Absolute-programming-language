AVStr source = "AVStr token = \"root\" \nAVStr typ = \"\" \nAVStr level = \"\" \nsecretup(token) \ntyp, level = info(token) \nout level \nout token \nsecretup(token) \ntyp, level = info(token) \nout level \nout token =self="
VTime output = vm.run_source(source)

out "VM secretup output:"

pick(output): line {
  out line
}

out "Done!"
