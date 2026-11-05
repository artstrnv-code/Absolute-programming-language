# Test APL bootstrap report facade.

AVStr ok_source = "AVInt x = input \nx += 1 \nout x"
AVStr stop_source = "out \"before\" \nstop \"done\" \nout \"after\""
AVStr fail_source = "out \"before\" \nfail \"bad\" \nout \"after\""

VTime ok_report = bootstrap.run_with_input_report(ok_source, ["4"])
VTime stop_report = bootstrap.run_report(stop_source)
VTime fail_report = bootstrap.run_report(fail_source)

out "Bootstrap reports:"
out get(ok_report, 0)
out get(get(ok_report, 1), 0)
out get(stop_report, 0)
out get(get(stop_report, 1), 0)
out get(get(stop_report, 1), 1)
out get(fail_report, 0)
out get(get(fail_report, 1), 0)
out get(get(fail_report, 1), 1)
out "Done!"
