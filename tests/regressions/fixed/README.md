# Fixed Regressions

These minimized sources preserve repaired failures. Their expected check or
runtime outcome is stated in each file and is asserted by tests at the owning
Rust/self-hosted layer.

| Severity | File | Repaired behavior |
| --- | --- | --- |
| P0 | `p0_prefixed_list_protection.apl` | `std::add/get/set/pop` retain list protection and block secret output |
| P0 | `p0_float_to_int.apl` | finite Float-to-Int conversion truncates toward zero |
| P0 | `p0_postfix_increment.apl` | postfix `++` and `--` lower to compound assignment |
| P1 | `p1_zero_slice_step.apl` | a zero slice step fails explicitly with `zero slice step` |
| P1 | `p1_while_variable_limit.apl` | a bounded loop accepts an integer expression as its limit |
| P1 | `p1_match_while_branches.apl` | `match`, `while limit`, and initial `while else` preserve cause-specific control flow |
| P1 | `p1_error_return.apl` | nested `ERR` termination preserves its message and explicit process code |
| P1 | `p1_nc_multiple_libraries.apl` | `nc` accepts comma-separated identifier namespaces with several included modules |
| P3 | `p3_unqualified_std_main.apl` | an open `std` namespace permits `AVInt main()` |
