use std::collections::{BTreeMap, BTreeSet};

pub const DESCRIPTOR_MAGIC: &str = "APL_PTR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    User,
    Runtime,
    Load,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Runtime => "runtime",
            Self::Load => "load",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Live,
    Marked,
    Freed,
    Unloaded,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Marked => "marked",
            Self::Freed => "freed",
            Self::Unloaded => "unloaded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiSignature {
    pub convention: String,
    pub argument_types: Vec<String>,
    pub result_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    pub id: i64,
    pub address: i64,
    pub origin: Origin,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopePolicy {
    pub marked: bool,
    pub collector: bool,
    pub flee: bool,
}

#[derive(Debug)]
struct Record {
    id: i64,
    address: usize,
    size: usize,
    origin: Origin,
    state: State,
    readable: bool,
    writable: bool,
    executable: bool,
    signature: Option<AbiSignature>,
    owned: Option<Box<[u8]>>,
}

#[derive(Debug)]
struct ScopeFrame {
    policy: ScopePolicy,
    allocations: Vec<i64>,
}

#[derive(Debug, Default)]
pub struct RawMemoryTable {
    next_id: i64,
    records: BTreeMap<i64, Record>,
    scopes: Vec<ScopeFrame>,
}

impl RawMemoryTable {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            records: BTreeMap::new(),
            scopes: Vec::new(),
        }
    }

    fn take_id(&mut self) -> Result<i64, String> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| "raw-memory identity counter overflow".to_owned())?;
        Ok(id)
    }

    pub fn malloc(&mut self, size: i64) -> Result<Descriptor, String> {
        let size = usize::try_from(size)
            .ok()
            .filter(|size| *size > 0)
            .ok_or_else(|| "memory::malloc size must be a positive Int".to_owned())?;
        let mut owned = vec![0_u8; size].into_boxed_slice();
        let address = owned.as_mut_ptr() as usize;
        let id = self.take_id()?;
        self.records.insert(
            id,
            Record {
                id,
                address,
                size,
                origin: Origin::User,
                state: State::Live,
                readable: true,
                writable: true,
                executable: false,
                signature: None,
                owned: Some(owned),
            },
        );
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.policy.marked)
        {
            scope.allocations.push(id);
        }
        self.descriptor(id)
    }

    pub fn load(
        &mut self,
        address: i64,
        size: i64,
        permissions: &str,
        signature: Option<AbiSignature>,
    ) -> Result<Descriptor, String> {
        self.register(address, size, permissions, signature, Origin::Load)
    }

    pub fn register_runtime(
        &mut self,
        address: i64,
        size: i64,
        permissions: &str,
        signature: Option<AbiSignature>,
    ) -> Result<Descriptor, String> {
        self.register(address, size, permissions, signature, Origin::Runtime)
    }

    fn register(
        &mut self,
        address: i64,
        size: i64,
        permissions: &str,
        signature: Option<AbiSignature>,
        origin: Origin,
    ) -> Result<Descriptor, String> {
        let address = usize::try_from(address)
            .ok()
            .filter(|address| *address != 0)
            .ok_or_else(|| "memory::load address must be a positive Int".to_owned())?;
        let size = usize::try_from(size)
            .ok()
            .filter(|size| *size > 0)
            .ok_or_else(|| "memory::load size must be a positive Int".to_owned())?;
        let readable = permissions.contains('r');
        let writable = permissions.contains('w');
        let executable = permissions.contains('x');
        if permissions.is_empty()
            || permissions
                .chars()
                .any(|permission| !matches!(permission, 'r' | 'w' | 'x'))
        {
            return Err("memory::load permissions must contain only r, w, and x".to_owned());
        }
        if executable != signature.is_some() {
            return Err(
                "memory::load executable permission requires exactly one ABI signature".to_owned(),
            );
        }
        if let Some(signature) = &signature {
            validate_signature(signature)?;
        }
        let id = self.take_id()?;
        self.records.insert(
            id,
            Record {
                id,
                address,
                size,
                origin,
                state: State::Live,
                readable,
                writable,
                executable,
                signature,
                owned: None,
            },
        );
        self.descriptor(id)
    }

    pub fn release_runtime(&mut self, id: i64) -> Result<(), String> {
        let record = self
            .records
            .get_mut(&id)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))?;
        if record.origin != Origin::Runtime {
            return Err(format!(
                "raw-memory identity `{id}` is not owned by the runtime"
            ));
        }
        if record.state != State::Live {
            return Err(format!(
                "raw-memory identity `{id}` is already {}",
                record.state.name()
            ));
        }
        record.state = State::Unloaded;
        Ok(())
    }

    pub fn descriptor(&self, id: i64) -> Result<Descriptor, String> {
        let record = self
            .records
            .get(&id)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))?;
        Ok(Descriptor {
            id: record.id,
            address: i64::try_from(record.address)
                .map_err(|_| "raw-memory address does not fit APL Int".to_owned())?,
            origin: record.origin,
        })
    }

    pub fn resolve_number(&self, value: i64) -> Result<i64, String> {
        let by_id = self.records.contains_key(&value).then_some(value);
        let by_addresses = usize::try_from(value)
            .ok()
            .map(|address| {
                self.records
                    .values()
                    .filter(|record| record.address == address)
                    .map(|record| record.id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if by_addresses.len() > 1 {
            return Err(format!(
                "ambiguous raw-memory address `{value}` has multiple historical identities"
            ));
        }
        let by_address = by_addresses.first().copied();
        match (by_id, by_address) {
            (Some(id), Some(address_id)) if id != address_id => Err(format!(
                "ambiguous raw-memory number `{value}` matches an ID and an address"
            )),
            (Some(id), _) | (_, Some(id)) => Ok(id),
            _ => Err(format!("unregistered raw-memory target `{value}`")),
        }
    }

    pub fn mark(&mut self, id: i64) -> Result<State, String> {
        let record = self
            .records
            .get_mut(&id)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))?;
        if record.state != State::Live {
            return Err(format!(
                "raw-memory identity `{id}` is already {}",
                record.state.name()
            ));
        }
        record.state = match record.origin {
            Origin::User => State::Marked,
            Origin::Load => State::Unloaded,
            Origin::Runtime => {
                return Err("runtime memory can only be marked by its owner".to_owned())
            }
        };
        Ok(record.state)
    }

    pub fn free_pending(&mut self) -> usize {
        let mut released = 0;
        for record in self.records.values_mut() {
            if record.origin == Origin::User && record.state == State::Marked {
                record.owned = None;
                record.state = State::Freed;
                released += 1;
            }
        }
        released
    }

    pub fn state(&self, id: i64) -> Result<State, String> {
        self.records
            .get(&id)
            .map(|record| record.state)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))
    }

    pub fn read(&self, id: i64, offset: i64, count: i64) -> Result<Vec<u8>, String> {
        let record = self.live_record(id)?;
        if !record.readable {
            return Err(format!("raw-memory identity `{id}` is not readable"));
        }
        let range = checked_range(record.size, offset, count)?;
        if let Some(owned) = &record.owned {
            return Ok(owned[range].to_vec());
        }
        read_external(record.address + range.start, range.len())
    }

    pub fn write(&mut self, id: i64, offset: i64, bytes: &[u8]) -> Result<(), String> {
        let record = self.live_record_mut(id)?;
        if !record.writable {
            return Err(format!("raw-memory identity `{id}` is not writable"));
        }
        let range = checked_range(
            record.size,
            offset,
            i64::try_from(bytes.len()).map_err(|_| "write is too large".to_owned())?,
        )?;
        if let Some(owned) = &mut record.owned {
            owned[range].copy_from_slice(bytes);
            return Ok(());
        }
        write_external(record.address + range.start, bytes)
    }

    pub fn call(&self, id: i64, arguments: &[i64]) -> Result<i64, String> {
        let record = self.live_record(id)?;
        if !record.executable {
            return Err(format!("raw-memory identity `{id}` is not executable"));
        }
        let signature = record
            .signature
            .as_ref()
            .ok_or_else(|| format!("raw-memory identity `{id}` has no ABI signature"))?;
        if arguments.len() != signature.argument_types.len() {
            return Err(format!(
                "memory::call expected {} arguments, got {}",
                signature.argument_types.len(),
                arguments.len()
            ));
        }
        let arguments = arguments
            .iter()
            .zip(&signature.argument_types)
            .map(|(value, value_type)| normalize_abi_argument(*value, value_type))
            .collect::<Result<Vec<_>, _>>()?;
        validate_executable_address(record.address)?;
        let result = call_integer_function(record.address, &signature.convention, &arguments)?;
        normalize_abi_result(result, &signature.result_type)
    }

    pub fn enter_scope(&mut self, policy: ScopePolicy) {
        self.scopes.push(ScopeFrame {
            policy,
            allocations: Vec::new(),
        });
    }

    pub fn exit_scope(&mut self, roots: &BTreeSet<i64>) -> Result<usize, String> {
        let scope = self
            .scopes
            .pop()
            .ok_or_else(|| "raw-memory scope stack underflow".to_owned())?;
        if scope.policy.marked {
            for id in scope.allocations {
                if self.state(id)? == State::Live {
                    self.mark(id)?;
                }
            }
        }
        if scope.policy.collector {
            let unreachable = self
                .records
                .values()
                .filter(|record| {
                    record.origin == Origin::User
                        && record.state == State::Live
                        && !roots.contains(&record.id)
                })
                .map(|record| record.id)
                .collect::<Vec<_>>();
            for id in unreachable {
                self.mark(id)?;
            }
        }
        Ok(if scope.policy.flee {
            self.free_pending()
        } else {
            0
        })
    }

    fn live_record(&self, id: i64) -> Result<&Record, String> {
        let record = self
            .records
            .get(&id)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))?;
        if record.state != State::Live {
            return Err(format!(
                "raw-memory identity `{id}` is {}",
                record.state.name()
            ));
        }
        Ok(record)
    }

    fn live_record_mut(&mut self, id: i64) -> Result<&mut Record, String> {
        let record = self
            .records
            .get_mut(&id)
            .ok_or_else(|| format!("unknown raw-memory identity `{id}`"))?;
        if record.state != State::Live {
            return Err(format!(
                "raw-memory identity `{id}` is {}",
                record.state.name()
            ));
        }
        Ok(record)
    }
}

fn checked_range(size: usize, offset: i64, count: i64) -> Result<std::ops::Range<usize>, String> {
    let offset =
        usize::try_from(offset).map_err(|_| "memory offset must be nonnegative".to_owned())?;
    let count = usize::try_from(count).map_err(|_| "memory size must be nonnegative".to_owned())?;
    let end = offset
        .checked_add(count)
        .filter(|end| *end <= size)
        .ok_or_else(|| "raw-memory access is outside the registered range".to_owned())?;
    Ok(offset..end)
}

fn validate_signature(signature: &AbiSignature) -> Result<(), String> {
    let convention_ok = if cfg!(windows) {
        matches!(signature.convention.as_str(), "system" | "win64")
    } else {
        matches!(signature.convention.as_str(), "c" | "sysv64")
    };
    if !convention_ok {
        return Err(format!(
            "ABI convention `{}` is not valid for this target",
            signature.convention
        ));
    }
    let maximum_arguments = if cfg!(windows) { 16 } else { 6 };
    if signature.argument_types.len() > maximum_arguments {
        return Err(format!(
            "memory::call supports at most {maximum_arguments} arguments on this target"
        ));
    }
    if let Some(invalid) = signature
        .argument_types
        .iter()
        .find(|value_type| !abi_type_is_valid(value_type, false))
    {
        return Err(format!("unsupported ABI argument type `{invalid}`"));
    }
    if !abi_type_is_valid(&signature.result_type, true) {
        return Err(format!(
            "unsupported ABI result type `{}`",
            signature.result_type
        ));
    }
    Ok(())
}

fn abi_type_is_valid(value_type: &str, result: bool) -> bool {
    matches!(
        value_type,
        "Int" | "I8" | "U8" | "I16" | "U16" | "I32" | "U32" | "I64" | "U64" | "Ptr" | "Bool"
    ) || (result && value_type == "Void")
}

fn normalize_abi_argument(value: i64, value_type: &str) -> Result<i64, String> {
    let value = match value_type {
        "Int" | "I64" | "U64" | "Ptr" => value,
        "I8" => i8::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI I8 value is out of range".to_owned())?,
        "U8" => u8::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI U8 value is out of range".to_owned())?,
        "I16" => i16::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI I16 value is out of range".to_owned())?,
        "U16" => u16::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI U16 value is out of range".to_owned())?,
        "I32" => i32::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI I32 value is out of range".to_owned())?,
        "U32" => u32::try_from(value)
            .map(i64::from)
            .map_err(|_| "ABI U32 value is out of range".to_owned())?,
        "Bool" => i64::from(value != 0),
        _ => return Err(format!("unsupported ABI type `{value_type}`")),
    };
    Ok(value)
}

fn normalize_abi_result(value: i64, value_type: &str) -> Result<i64, String> {
    let value = match value_type {
        "Int" | "I64" | "U64" | "Ptr" => value,
        "I8" => i64::from(value as i8),
        "U8" => i64::from(value as u8),
        "I16" => i64::from(value as i16),
        "U16" => i64::from(value as u16),
        "I32" => i64::from(value as i32),
        "U32" => i64::from(value as u32),
        "Bool" => i64::from(value != 0),
        "Void" => 0,
        _ => return Err(format!("unsupported ABI type `{value_type}`")),
    };
    Ok(value)
}

fn call_integer_function(address: usize, convention: &str, args: &[i64]) -> Result<i64, String> {
    if address == 0 {
        return Err("cannot call the null address".to_owned());
    }
    #[cfg(windows)]
    unsafe {
        macro_rules! invoke {
            ($type:ty, $($arg:expr),*) => {{
                let function: $type = std::mem::transmute(address);
                function($($arg),*)
            }};
        }
        let result = match args {
            [] => invoke!(unsafe extern "system" fn() -> i64,),
            [a] => invoke!(unsafe extern "system" fn(i64) -> i64, *a),
            [a, b] => invoke!(unsafe extern "system" fn(i64, i64) -> i64, *a, *b),
            [a, b, c] => invoke!(unsafe extern "system" fn(i64, i64, i64) -> i64, *a, *b, *c),
            [a, b, c, d] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d
            ),
            [a, b, c, d, e] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e
            ),
            [a, b, c, d, e, f] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f
            ),
            [a, b, c, d, e, f, g] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g
            ),
            [a, b, c, d, e, f, g, h] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h
            ),
            [a, b, c, d, e, f, g, h, i] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i
            ),
            [a, b, c, d, e, f, g, h, i, j] => invoke!(
                unsafe extern "system" fn(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j
            ),
            [a, b, c, d, e, f, g, h, i, j, k] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k
            ),
            [a, b, c, d, e, f, g, h, i, j, k, l] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k,
                *l
            ),
            [a, b, c, d, e, f, g, h, i, j, k, l, m] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k,
                *l,
                *m
            ),
            [a, b, c, d, e, f, g, h, i, j, k, l, m, n] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k,
                *l,
                *m,
                *n
            ),
            [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k,
                *l,
                *m,
                *n,
                *o
            ),
            [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p] => invoke!(
                unsafe extern "system" fn(
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                    i64,
                ) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f,
                *g,
                *h,
                *i,
                *j,
                *k,
                *l,
                *m,
                *n,
                *o,
                *p
            ),
            _ => {
                return Err("memory::call supports at most sixteen arguments on Windows".to_owned())
            }
        };
        let _ = convention;
        Ok(result)
    }
    #[cfg(not(windows))]
    unsafe {
        macro_rules! invoke {
            ($type:ty, $($arg:expr),*) => {{
                let function: $type = std::mem::transmute(address);
                function($($arg),*)
            }};
        }
        let result = match args {
            [] => invoke!(unsafe extern "C" fn() -> i64,),
            [a] => invoke!(unsafe extern "C" fn(i64) -> i64, *a),
            [a, b] => invoke!(unsafe extern "C" fn(i64, i64) -> i64, *a, *b),
            [a, b, c] => invoke!(unsafe extern "C" fn(i64, i64, i64) -> i64, *a, *b, *c),
            [a, b, c, d] => invoke!(
                unsafe extern "C" fn(i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d
            ),
            [a, b, c, d, e] => invoke!(
                unsafe extern "C" fn(i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e
            ),
            [a, b, c, d, e, f] => invoke!(
                unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64,
                *a,
                *b,
                *c,
                *d,
                *e,
                *f
            ),
            _ => return Err("memory::call supports at most six arguments".to_owned()),
        };
        let _ = convention;
        Ok(result)
    }
}

#[cfg(windows)]
fn read_external(address: usize, count: usize) -> Result<Vec<u8>, String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn ReadProcessMemory(
            process: *mut core::ffi::c_void,
            base: *const core::ffi::c_void,
            buffer: *mut core::ffi::c_void,
            size: usize,
            read: *mut usize,
        ) -> i32;
    }
    let mut bytes = vec![0_u8; count];
    let mut read = 0;
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const _,
            bytes.as_mut_ptr().cast(),
            count,
            &mut read,
        )
    };
    if ok == 0 || read != count {
        return Err("operating system rejected the external memory read".to_owned());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn write_external(address: usize, bytes: &[u8]) -> Result<(), String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn WriteProcessMemory(
            process: *mut core::ffi::c_void,
            base: *mut core::ffi::c_void,
            buffer: *const core::ffi::c_void,
            size: usize,
            written: *mut usize,
        ) -> i32;
    }
    let mut written = 0;
    let ok = unsafe {
        WriteProcessMemory(
            GetCurrentProcess(),
            address as *mut _,
            bytes.as_ptr().cast(),
            bytes.len(),
            &mut written,
        )
    };
    if ok == 0 || written != bytes.len() {
        return Err("operating system rejected the external memory write".to_owned());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct IoVec {
    base: *mut core::ffi::c_void,
    length: usize,
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn getpid() -> i32;
    fn process_vm_readv(
        pid: i32,
        local: *const IoVec,
        local_count: usize,
        remote: *const IoVec,
        remote_count: usize,
        flags: usize,
    ) -> isize;
    fn process_vm_writev(
        pid: i32,
        local: *const IoVec,
        local_count: usize,
        remote: *const IoVec,
        remote_count: usize,
        flags: usize,
    ) -> isize;
}

#[cfg(target_os = "linux")]
fn read_external(address: usize, count: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0_u8; count];
    let local = IoVec {
        base: bytes.as_mut_ptr().cast(),
        length: count,
    };
    let remote = IoVec {
        base: address as *mut _,
        length: count,
    };
    let read = unsafe { process_vm_readv(getpid(), &local, 1, &remote, 1, 0) };
    if read != count as isize {
        return Err("operating system rejected the external memory read".to_owned());
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn write_external(address: usize, bytes: &[u8]) -> Result<(), String> {
    let local = IoVec {
        base: bytes.as_ptr() as *mut _,
        length: bytes.len(),
    };
    let remote = IoVec {
        base: address as *mut _,
        length: bytes.len(),
    };
    let written = unsafe { process_vm_writev(getpid(), &local, 1, &remote, 1, 0) };
    if written != bytes.len() as isize {
        return Err("operating system rejected the external memory write".to_owned());
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn read_external(_address: usize, _count: usize) -> Result<Vec<u8>, String> {
    Err("external memory reads are not implemented for this target".to_owned())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn write_external(_address: usize, _bytes: &[u8]) -> Result<(), String> {
    Err("external memory writes are not implemented for this target".to_owned())
}

#[cfg(windows)]
fn validate_executable_address(address: usize) -> Result<(), String> {
    #[repr(C)]
    struct MemoryBasicInformation {
        base_address: *mut core::ffi::c_void,
        allocation_base: *mut core::ffi::c_void,
        allocation_protect: u32,
        partition_id: u16,
        region_size: usize,
        state: u32,
        protect: u32,
        kind: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn VirtualQuery(
            address: *const core::ffi::c_void,
            information: *mut MemoryBasicInformation,
            length: usize,
        ) -> usize;
    }
    let mut information = std::mem::MaybeUninit::<MemoryBasicInformation>::uninit();
    let read = unsafe {
        VirtualQuery(
            address as *const _,
            information.as_mut_ptr(),
            std::mem::size_of::<MemoryBasicInformation>(),
        )
    };
    if read != std::mem::size_of::<MemoryBasicInformation>() {
        return Err("operating system rejected the executable address".to_owned());
    }
    let information = unsafe { information.assume_init() };
    let executable = matches!(information.protect & 0xff, 0x10 | 0x20 | 0x40 | 0x80);
    if information.state != 0x1000 || !executable {
        return Err("registered address is not executable in the current process".to_owned());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_executable_address(address: usize) -> Result<(), String> {
    let maps = std::fs::read_to_string("/proc/self/maps")
        .map_err(|_| "cannot inspect executable mappings for the current process".to_owned())?;
    for line in maps.lines() {
        let mut fields = line.split_whitespace();
        let Some(range) = fields.next() else { continue };
        let Some(permissions) = fields.next() else {
            continue;
        };
        let Some((start, end)) = range.split_once('-') else {
            continue;
        };
        let Some(start) = usize::from_str_radix(start, 16).ok() else {
            continue;
        };
        let Some(end) = usize::from_str_radix(end, 16).ok() else {
            continue;
        };
        if address >= start && address < end && permissions.as_bytes().get(2) == Some(&b'x') {
            return Ok(());
        }
    }
    Err("registered address is not executable in the current process".to_owned())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn validate_executable_address(_address: usize) -> Result<(), String> {
    Err("indirect calls are not implemented for this target".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    unsafe extern "system" fn add(left: i64, right: i64) -> i64 {
        left + right
    }

    #[cfg(not(windows))]
    unsafe extern "C" fn add(left: i64, right: i64) -> i64 {
        left + right
    }

    #[cfg(windows)]
    unsafe extern "system" fn add_twelve(
        a: i64,
        b: i64,
        c: i64,
        d: i64,
        e: i64,
        f: i64,
        g: i64,
        h: i64,
        i: i64,
        j: i64,
        k: i64,
        l: i64,
    ) -> i64 {
        a + b + c + d + e + f + g + h + i + j + k + l
    }

    #[test]
    fn owned_memory_is_revoked_before_it_is_released() {
        let mut table = RawMemoryTable::new();
        let descriptor = table.malloc(4).unwrap();
        table.write(descriptor.id, 0, &[1, 2, 3, 4]).unwrap();
        assert_eq!(table.read(descriptor.id, 1, 2).unwrap(), [2, 3]);
        assert_eq!(table.mark(descriptor.id), Ok(State::Marked));
        assert!(table.read(descriptor.id, 0, 1).is_err());
        assert_eq!(table.free_pending(), 1);
        assert_eq!(table.state(descriptor.id), Ok(State::Freed));

        let next = table.malloc(1).unwrap();
        assert!(next.id > descriptor.id);
        assert_eq!(table.descriptor(descriptor.id).unwrap().id, descriptor.id);
    }

    #[test]
    fn loaded_memory_is_unregistered_but_never_freed() {
        let mut table = RawMemoryTable::new();
        let descriptor = table.load(4096, 16, "r", None).unwrap();
        assert_eq!(table.mark(descriptor.id), Ok(State::Unloaded));
        assert_eq!(table.free_pending(), 0);
        assert_eq!(table.state(descriptor.id), Ok(State::Unloaded));

        table.load(4096, 16, "r", None).unwrap();
        assert!(table.resolve_number(4096).is_err());
    }

    #[test]
    fn runtime_capabilities_can_only_be_released_by_their_owner() {
        let mut table = RawMemoryTable::new();
        let convention = if cfg!(windows) { "win64" } else { "sysv64" };
        let descriptor = table
            .register_runtime(
                add as *const () as usize as i64,
                1,
                "rx",
                Some(AbiSignature {
                    convention: convention.to_owned(),
                    argument_types: vec!["Int".to_owned(), "Int".to_owned()],
                    result_type: "Int".to_owned(),
                }),
            )
            .unwrap();

        assert_eq!(descriptor.origin, Origin::Runtime);
        assert_eq!(table.call(descriptor.id, &[19, 23]), Ok(42));
        assert_eq!(
            table.mark(descriptor.id).unwrap_err(),
            "runtime memory can only be marked by its owner"
        );
        table.release_runtime(descriptor.id).unwrap();
        assert_eq!(table.state(descriptor.id), Ok(State::Unloaded));
        assert!(table.call(descriptor.id, &[19, 23]).is_err());
    }

    #[test]
    fn loaded_memory_reads_and_writes_the_registered_external_region() {
        let mut external = [1_u8, 2, 3, 4];
        let mut table = RawMemoryTable::new();
        let descriptor = table
            .load(
                external.as_mut_ptr() as usize as i64,
                external.len() as i64,
                "rw",
                None,
            )
            .unwrap();

        assert_eq!(table.read(descriptor.id, 1, 2).unwrap(), [2, 3]);
        table.write(descriptor.id, 2, &[9, 8]).unwrap();
        assert_eq!(external, [1, 2, 9, 8]);
    }

    #[test]
    fn marked_scope_revokes_only_its_own_allocations() {
        let mut table = RawMemoryTable::new();
        let outer = table.malloc(1).unwrap();
        table.enter_scope(ScopePolicy {
            marked: true,
            ..ScopePolicy::default()
        });
        let inner = table.malloc(1).unwrap();
        table.exit_scope(&BTreeSet::new()).unwrap();
        assert_eq!(table.state(outer.id), Ok(State::Live));
        assert_eq!(table.state(inner.id), Ok(State::Marked));
    }

    #[test]
    fn indirect_calls_require_registered_execute_permission_and_exact_abi() {
        let mut table = RawMemoryTable::new();
        let convention = if cfg!(windows) { "win64" } else { "sysv64" };
        let target = table
            .load(
                add as *const () as usize as i64,
                1,
                "rx",
                Some(AbiSignature {
                    convention: convention.to_owned(),
                    argument_types: vec!["Int".to_owned(), "Int".to_owned()],
                    result_type: "Int".to_owned(),
                }),
            )
            .unwrap();
        assert_eq!(table.call(target.id, &[20, 22]), Ok(42));
        assert!(table.call(target.id, &[1]).is_err());

        let allocation = table.malloc(8).unwrap();
        assert!(table.call(allocation.id, &[]).is_err());
        assert!(table.resolve_number(123_456_789).is_err());
    }

    #[test]
    fn abi_values_validate_arguments_and_truncate_results() {
        assert_eq!(normalize_abi_argument(255, "U8"), Ok(255));
        assert!(normalize_abi_argument(256, "U8").is_err());
        assert_eq!(normalize_abi_argument(-1, "Bool"), Ok(1));
        assert_eq!(normalize_abi_result(255, "I8"), Ok(-1));
        assert_eq!(normalize_abi_result(-1, "U32"), Ok(4_294_967_295));
        assert_eq!(normalize_abi_result(42, "Void"), Ok(0));
    }

    #[test]
    #[cfg(windows)]
    fn win64_calls_use_registers_and_stack_for_twelve_arguments() {
        let mut table = RawMemoryTable::new();
        let target = table
            .load(
                add_twelve as *const () as usize as i64,
                1,
                "rx",
                Some(AbiSignature {
                    convention: "win64".to_owned(),
                    argument_types: vec!["I64".to_owned(); 12],
                    result_type: "I64".to_owned(),
                }),
            )
            .unwrap();
        assert_eq!(
            table.call(target.id, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            Ok(78)
        );
    }
}
