$lines = Get-Content 'D:\veles\apl\crates\apl_runtime\src\lib.rs'
Write-Host "Lines: $($lines.Count)"

$evalForTypeEnd = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match 'fn eval_for_type\(') {
        $bc = 0
        for ($j = $i; $j -lt $lines.Count; $j++) {
            $bc += ($lines[$j].ToCharArray() | Where-Object {$_ -eq "{" }).Count
            $bc -= ($lines[$j].ToCharArray() | Where-Object {$_ -eq "}"}).Count
            if ($bc -eq 0 -and $j -gt $i) { $evalForTypeEnd = $j; break }
        }
        break
    }
}

Write-Host "End: $evalForTypeEnd"

$evalCall = "\n\n    fn eval_call(&mut self, name: &str, args: Vec<RuntimeValue>, arg_forms: &[ExpressionForm]) -> Result<RuntimeValue, RuntimeError> {
        if let Some(value_type) = conversion_type(name) {
            let value = args[0].clone();
            return Ok(value.convert(value_type));
        }
        match name {
            \"get\" => { let target = args[0].clone(); let index = args[1].clone(); self.eval_index(target, index) }
            \"pop\" => { let ExpressionForm::Variable(name) = &arg_forms[0] else { return Err(RuntimeError::Message(\"pop expects list name\".to_owned())); }; self.pop_list(name) }
            \"split\" => { let target = args[0].clone(); let delim = args[1].clone(); let RuntimeData::Str(text) = &target.data else { return Ok(RuntimeValue::none(target.effective_protection())); }; let RuntimeData::Str(separator) = &delim.data else { return Ok(RuntimeValue::none(target.effective_protection())); }; if separator.is_empty() { return Ok(RuntimeValue::none(target.effective_protection())); } let parts: Vec<RuntimeValue> = text.split(separator).map(|s| RuntimeValue::str(s.to_owned())).collect(); let protection = parts.iter().map(RuntimeValue::effective_protection).fold(ProtectionLevel::Av, ProtectionLevel::max); Ok(RuntimeValue::list(parts).with_protection(protection)) }
            \"len\" => { let target = args[0].clone(); let protection = target.effective_protection(); let length = match &target.data { RuntimeData::Str(s) => s.chars().count() as i64, RuntimeData::List(values) => values.len() as i64, RuntimeData::Bytes(b) => b.len() as i64, _ => return Ok(RuntimeValue::none(protection)) }; Ok(RuntimeValue::new(RuntimeData::Int(length), protection)) }
            \"contains\" => { let target = args[0].clone(); let sub = args[1].clone(); let RuntimeData::Str(text) = &target.data else { return Ok(RuntimeValue::none(target.effective_protection())); }; let RuntimeData::Str(substr) = &sub.data else { return Ok(RuntimeValue::none(target.effective_protection())); }; let protection = target.effective_protection(); Ok(RuntimeValue::new(RuntimeData::Bool(text.contains(substr)), protection)) }
            _ => { let Some(function) = self.program.and_then(|program| program.functions.get(name)).cloned() else { return Err(RuntimeError::Message(format!(\"unknown function `{name}`\"))); }; self.with_scope(|runtime| { for (param, value) in function.params.iter().zip(args) { runtime.declare_vtime(param.clone(), value); } match runtime.exec_block(function.body)? { Flow::Return(value) => Ok(value), Flow::None | Flow::Stop => Ok(RuntimeValue::none(ProtectionLevel::Av)), Flow::Break => Err(RuntimeError::Message(\"break escaped function\".to_owned())), Flow::Continue => Err(RuntimeError::Message(\"continue escaped function\".to_owned())), } }) }
        }
    }
"

$prefix = $lines[0..$evalForTypeEnd]
$suffix = $lines[($evalForTypeEnd+1)..($lines.Count-1)]
$newLines = $prefix + $evalCall + $suffix
Set-Content -Path 'D:\veles\apl\crates\apl_runtime\src\lib.rs' -Value $newLines
Write-Host "New: $($newLines.Count)"
