AVStr ok_source = "AVStr typ = \"\" \nAVStr level = \"\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ \nout level"
AVStr missing_target_source = "out \"before\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ"
AVStr secret_target_source = "ASVStr typ = \"\" \nAVStr level = \"\" \nAVStr public = \"x\" \ntyp, level = info(public) \nout level"
AVStr vtime_source = "AVStr typ = \"\" \nAVStr level = \"\" \nVTime tmp = 1 \ntyp, level = info(tmp) \nout typ"
VTime ok_output = vm.run_source(ok_source)
VTime missing_target_output = vm.run_source(missing_target_source)
VTime secret_target_output = vm.run_source(secret_target_source)
VTime vtime_output = vm.run_source(vtime_source)

out "VM info guards output:"
out get(ok_output, 0)
out get(ok_output, 1)
out len(missing_target_output)
out get(missing_target_output, 0)
out len(secret_target_output)
out len(vtime_output)
out "Done!"
