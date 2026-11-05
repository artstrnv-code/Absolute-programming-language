AVStr assign_source = "out \"before\" \nmissing = 4 \nout \"after\""
AVStr secretup_source = "out \"start\" \nsecretup(missing) \nout \"after\""
VTime assign_output = vm.run_source(assign_source)
VTime secretup_output = vm.run_source(secretup_source)

out "VM unknown targets output:"
out len(assign_output)
out get(assign_output, 0)
out get(assign_output, 1)
out len(secretup_output)
out get(secretup_output, 0)
out get(secretup_output, 1)
out "Done!"
