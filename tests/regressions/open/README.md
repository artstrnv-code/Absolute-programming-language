# Open Regressions

These files intentionally reproduce current bugs and are not passing examples.

| Severity | File | Current bug |
| --- | --- | --- |
| P1 | `p1_missing_semantic_position.apl` | The portable checker reports `unknown variable missing` without the expected `semantic error at 2:3` source position |

Keep each source minimal. A fix is incomplete until the reproduction becomes an
automated test at the narrowest owning layer.

Reproduce from the repository root:

```powershell
.\dist\package\apl.exe check .\tests\regressions\open\p1_missing_semantic_position.apl
```
