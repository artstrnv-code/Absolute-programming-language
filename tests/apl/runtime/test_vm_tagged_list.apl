AVStr source = "AVStr typ = \"\" \nAVStr level = \"\" \nList items = [1:SASV, \"public\"] \nadd(items, \"hidden\":ASV) \ntyp, level = info(items) \nout typ \nout level \nout items \nout get(items, 0) \nout get(items, 1) \nout get(items, 2) \npick(items): item { \nout item \n}"
VTime output = vm.run_source(source)

out "VM tagged list output:"

pick(output): line {
  out line
}

out "Done!"
