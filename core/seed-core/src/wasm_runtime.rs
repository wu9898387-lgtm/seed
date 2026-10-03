use std::collections::BTreeMap;

use wasmi::{
    Caller, Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::{
    capability::Capability,
    plugin::{PluginManifest, PluginPermission},
    PLUGIN_API_VERSION,
};

pub const DEFAULT_PLUGIN_FUEL: u64 = 50_000;
pub const DEFAULT_PLUGIN_MEMORY_BYTES: usize = 64 * 1024;
pub const DEFAULT_PLUGIN_STATE_ENTRIES: usize = 1024;
pub const DEFAULT_PLUGIN_HOST_CALLS_PER_INVOCATION: u32 = 1024;
pub const SUPPORTED_PLUGIN_MANIFEST_VERSION: u16 = 1;

pub const HOST_DENY: i32 = 0;
pub const HOST_ALLOW: i32 = 1;

pub const CAPABILITY_MEMBER_REMOVE: i32 = 1;

/// Minimal Phase-0 host state.
///
/// Every plugin instance receives its own Store and therefore its own scoped
/// state map. The runtime intentionally exposes no filesystem, network, clock,
/// randomness, database handle, or private-key host API.
#[derive(Debug)]
struct PluginHostState {
    manifest: PluginManifest,
    state: BTreeMap<i32, i32>,
    host_calls_remaining: u32,
    limits: StoreLimits,
}

impl PluginHostState {
    fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            state: BTreeMap::new(),
            host_calls_remaining: DEFAULT_PLUGIN_HOST_CALLS_PER_INVOCATION,
            limits: StoreLimitsBuilder::new()
                .memory_size(DEFAULT_PLUGIN_MEMORY_BYTES)
                .instances(1)
                .memories(1)
                .tables(1)
                .table_elements(1024)
                .trap_on_grow_failure(true)
                .build(),
        }
    }

    fn scoped_storage_allowed(&self) -> bool {
        self.manifest
            .permissions
            .iter()
            .any(|permission| matches!(permission, PluginPermission::ScopedStorage))
    }

    fn put_scoped_state(&mut self, key: i32, value: i32) -> bool {
        if !self.scoped_storage_allowed() {
            return false;
        }
        if !self.state.contains_key(&key) && self.state.len() >= DEFAULT_PLUGIN_STATE_ENTRIES {
            return false;
        }
        self.state.insert(key, value);
        true
    }

    fn reset_host_calls(&mut self, budget: u32) {
        self.host_calls_remaining = budget;
    }

    fn consume_host_call(&mut self) -> bool {
        if self.host_calls_remaining == 0 {
            return false;
        }
        self.host_calls_remaining -= 1;
        true
    }

    fn capability_allowed(&self, code: i32) -> bool {
        let capability = match code {
            CAPABILITY_MEMBER_REMOVE => Capability::MemberRemove,
            _ => return false,
        };
        self.manifest.can_evaluate(&capability)
    }
}

/// Replaceable WebAssembly engine adapter used only by the Phase-0 spike.
///
/// The surrounding Seed plugin model depends on the Host ABI and permission
/// checks, not on Wasmi-specific objects.
#[derive(Clone)]
pub struct WasmiPluginRuntime {
    engine: Engine,
}

impl Default for WasmiPluginRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmiPluginRuntime {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.consume_fuel(true).allow_start_fn(false);
        Self {
            engine: Engine::new(&config),
        }
    }

    pub fn instantiate(
        &self,
        manifest: PluginManifest,
        wasm: &[u8],
    ) -> Result<WasmiPluginInstance, wasmi::Error> {
        self.instantiate_with_budget(
            manifest,
            wasm,
            DEFAULT_PLUGIN_FUEL,
            DEFAULT_PLUGIN_HOST_CALLS_PER_INVOCATION,
        )
    }

    pub fn instantiate_with_fuel(
        &self,
        manifest: PluginManifest,
        wasm: &[u8],
        fuel: u64,
    ) -> Result<WasmiPluginInstance, wasmi::Error> {
        self.instantiate_with_budget(
            manifest,
            wasm,
            fuel,
            DEFAULT_PLUGIN_HOST_CALLS_PER_INVOCATION,
        )
    }

    pub fn instantiate_with_budget(
        &self,
        manifest: PluginManifest,
        wasm: &[u8],
        fuel: u64,
        host_calls: u32,
    ) -> Result<WasmiPluginInstance, wasmi::Error> {
        if manifest.manifest_version != SUPPORTED_PLUGIN_MANIFEST_VERSION {
            return Err(wasmi::Error::new("unsupported Seed plugin manifest version"));
        }
        if manifest.plugin_api_version != PLUGIN_API_VERSION {
            return Err(wasmi::Error::new("unsupported Seed plugin API version"));
        }

        let module = Module::new(&self.engine, wasm)?;
        let mut store = Store::new(&self.engine, PluginHostState::new(manifest));
        store.limiter(|state| &mut state.limits);
        store.set_fuel(fuel)?;

        let mut linker = <Linker<PluginHostState>>::new(&self.engine);
        linker.func_wrap(
            "seed",
            "capability_allowed",
            |mut caller: Caller<'_, PluginHostState>, code: i32| -> i32 {
                if !caller.data_mut().consume_host_call() {
                    return HOST_DENY;
                }
                if caller.data().capability_allowed(code) {
                    HOST_ALLOW
                } else {
                    HOST_DENY
                }
            },
        )?;
        linker.func_wrap(
            "seed",
            "state_put",
            |mut caller: Caller<'_, PluginHostState>, key: i32, value: i32| -> i32 {
                if !caller.data_mut().consume_host_call() {
                    return HOST_DENY;
                }
                if caller.data_mut().put_scoped_state(key, value) {
                    HOST_ALLOW
                } else {
                    HOST_DENY
                }
            },
        )?;
        linker.func_wrap(
            "seed",
            "state_get",
            |mut caller: Caller<'_, PluginHostState>, key: i32| -> i32 {
                if !caller.data_mut().consume_host_call() {
                    return 0;
                }
                if !caller.data().scoped_storage_allowed() {
                    return 0;
                }
                caller.data().state.get(&key).copied().unwrap_or(0)
            },
        )?;

        let instance = linker.instantiate_and_start(&mut store, &module)?;
        Ok(WasmiPluginInstance {
            store,
            instance,
            fuel_per_call: fuel,
            host_calls_per_call: host_calls,
        })
    }
}

pub struct WasmiPluginInstance {
    store: Store<PluginHostState>,
    instance: Instance,
    fuel_per_call: u64,
    host_calls_per_call: u32,
}

impl WasmiPluginInstance {
    pub fn call_i32(&mut self, export: &str) -> Result<i32, wasmi::Error> {
        let function = self
            .instance
            .get_typed_func::<(), i32>(&self.store, export)?;
        let state_before = self.prepare_invocation()?;
        match function.call(&mut self.store, ()) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.store.data_mut().state = state_before;
                Err(error)
            }
        }
    }

    pub fn call_unit(&mut self, export: &str) -> Result<(), wasmi::Error> {
        let function = self
            .instance
            .get_typed_func::<(), ()>(&self.store, export)?;
        let state_before = self.prepare_invocation()?;
        match function.call(&mut self.store, ()) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.store.data_mut().state = state_before;
                Err(error)
            }
        }
    }

    fn prepare_invocation(&mut self) -> Result<BTreeMap<i32, i32>, wasmi::Error> {
        self.store.set_fuel(self.fuel_per_call)?;
        self.store
            .data_mut()
            .reset_host_calls(self.host_calls_per_call);
        Ok(self.store.data().state.clone())
    }

    pub fn remaining_fuel(&self) -> Result<u64, wasmi::Error> {
        self.store.get_fuel()
    }

    pub fn scoped_state(&self, key: i32) -> Option<i32> {
        self.store.data().state.get(&key).copied()
    }

    pub fn remaining_host_calls(&self) -> u32 {
        self.store.data().host_calls_remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        id::PluginId,
        plugin::{PluginPermission, PluginVersion},
    };

    // (import "seed" "capability_allowed" (func (param i32) (result i32)))
    // (func (export "on_load") (result i32)
    //   i32.const 1
    //   call 0)
    const CAPABILITY_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0a, 0x02, 0x60, 0x01, 0x7f, 0x01,
        0x7f, 0x60, 0x00, 0x01, 0x7f, 0x02, 0x1b, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64, 0x12, 0x63,
        0x61, 0x70, 0x61, 0x62, 0x69, 0x6c, 0x69, 0x74, 0x79, 0x5f, 0x61, 0x6c, 0x6c, 0x6f, 0x77,
        0x65, 0x64, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x0b, 0x01, 0x07, 0x6f, 0x6e, 0x5f,
        0x6c, 0x6f, 0x61, 0x64, 0x00, 0x01, 0x0a, 0x08, 0x01, 0x06, 0x00, 0x41, 0x01, 0x10, 0x00,
        0x0b,
    ];

    // Imports seed.state_put(i32, i32)->i32 and seed.state_get(i32)->i32.
    // on_load writes 42 under key 7 and returns the read value.
    const STATE_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x10, 0x03, 0x60, 0x02, 0x7f, 0x7f,
        0x01, 0x7f, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x01, 0x7f, 0x02, 0x23, 0x02, 0x04,
        0x73, 0x65, 0x65, 0x64, 0x09, 0x73, 0x74, 0x61, 0x74, 0x65, 0x5f, 0x70, 0x75, 0x74, 0x00,
        0x00, 0x04, 0x73, 0x65, 0x65, 0x64, 0x09, 0x73, 0x74, 0x61, 0x74, 0x65, 0x5f, 0x67, 0x65,
        0x74, 0x00, 0x01, 0x03, 0x02, 0x01, 0x02, 0x07, 0x0b, 0x01, 0x07, 0x6f, 0x6e, 0x5f, 0x6c,
        0x6f, 0x61, 0x64, 0x00, 0x02, 0x0a, 0x0f, 0x01, 0x0d, 0x00, 0x41, 0x07, 0x41, 0x2a, 0x10,
        0x00, 0x1a, 0x41, 0x07, 0x10, 0x01, 0x0b,
    ];

    // (func (export "run") (loop br 0))
    const INFINITE_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x00, 0x0a, 0x09, 0x01,
        0x07, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x0b,
    ];

    // (memory 2) => 128 KiB initial memory, above the 64 KiB host limit.
    const OVERSIZED_MEMORY_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, 0x02,
    ];

    // (memory 1) (func (export "grow") (result i32) i32.const 1 memory.grow)
    // The initial 64 KiB fits exactly; growing by one page must hit the Store limit.
    const GROW_MEMORY_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
        0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x01, 0x07, 0x08, 0x01, 0x04, 0x67, 0x72,
        0x6f, 0x77, 0x00, 0x00, 0x0a, 0x08, 0x01, 0x06, 0x00, 0x41, 0x01, 0x40, 0x00, 0x0b,
    ];

    // Calls capability_allowed three times and returns the third result.
    const HOST_CALL_BURST_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0a, 0x02, 0x60, 0x01, 0x7f, 0x01,
        0x7f, 0x60, 0x00, 0x01, 0x7f, 0x02, 0x1b, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64, 0x12, 0x63,
        0x61, 0x70, 0x61, 0x62, 0x69, 0x6c, 0x69, 0x74, 0x79, 0x5f, 0x61, 0x6c, 0x6c, 0x6f, 0x77,
        0x65, 0x64, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e,
        0x00, 0x01, 0x0a, 0x12, 0x01, 0x10, 0x00, 0x41, 0x01, 0x10, 0x00, 0x1a, 0x41, 0x01, 0x10,
        0x00, 0x1a, 0x41, 0x01, 0x10, 0x00, 0x0b,
    ];

    // Calls state_put(7, 42) and then traps with unreachable.
    const STATE_THEN_TRAP_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0a, 0x02, 0x60, 0x02, 0x7f, 0x7f,
        0x01, 0x7f, 0x60, 0x00, 0x00, 0x02, 0x12, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64, 0x09, 0x73,
        0x74, 0x61, 0x74, 0x65, 0x5f, 0x70, 0x75, 0x74, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07,
        0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x01, 0x0a, 0x0c, 0x01, 0x0a, 0x00, 0x41, 0x07,
        0x41, 0x2a, 0x10, 0x00, 0x1a, 0x00, 0x0b,
    ];

    // Attempts to import a private-key host API which Seed never links.
    const ROOT_KEY_IMPORT_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
        0x02, 0x19, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64, 0x10, 0x72, 0x6f, 0x6f, 0x74, 0x5f, 0x70,
        0x72, 0x69, 0x76, 0x61, 0x74, 0x65, 0x5f, 0x6b, 0x65, 0x79, 0x00, 0x00,
    ];

    // Attempts to import a WASI network-like API. No WASI namespace is linked.
    const WASI_IMPORT_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x02,
        0x24, 0x01, 0x16, 0x77, 0x61, 0x73, 0x69, 0x5f, 0x73, 0x6e, 0x61, 0x70, 0x73, 0x68, 0x6f,
        0x74, 0x5f, 0x70, 0x72, 0x65, 0x76, 0x69, 0x65, 0x77, 0x31, 0x09, 0x73, 0x6f, 0x63, 0x6b,
        0x5f, 0x6f, 0x70, 0x65, 0x6e, 0x00, 0x00,
    ];

    fn manifest() -> PluginManifest {
        PluginManifest::new(
            PluginId::from_bytes([0x71; 32]),
            PluginVersion {
                major: 0,
                minor: 1,
                patch: 0,
            },
        )
    }

    #[test]
    fn incompatible_manifest_or_plugin_api_version_is_rejected() {
        let runtime = WasmiPluginRuntime::new();

        let mut unsupported_manifest = manifest();
        unsupported_manifest.manifest_version = SUPPORTED_PLUGIN_MANIFEST_VERSION + 1;
        assert!(runtime
            .instantiate(unsupported_manifest, CAPABILITY_PLUGIN)
            .is_err());

        let mut unsupported_api = manifest();
        unsupported_api.plugin_api_version = PLUGIN_API_VERSION + 1;
        assert!(runtime
            .instantiate(unsupported_api, CAPABILITY_PLUGIN)
            .is_err());
    }

    #[test]
    fn capability_host_call_is_denied_without_manifest_permission() {
        let runtime = WasmiPluginRuntime::new();
        let mut plugin = runtime.instantiate(manifest(), CAPABILITY_PLUGIN).unwrap();
        assert_eq!(plugin.call_i32("on_load").unwrap(), HOST_DENY);
    }

    #[test]
    fn capability_host_call_is_allowed_only_when_declared() {
        let runtime = WasmiPluginRuntime::new();
        let mut manifest = manifest();
        manifest
            .permissions
            .push(PluginPermission::EvaluateCapability(
                Capability::MemberRemove,
            ));

        let mut plugin = runtime.instantiate(manifest, CAPABILITY_PLUGIN).unwrap();
        assert_eq!(plugin.call_i32("on_load").unwrap(), HOST_ALLOW);
    }

    #[test]
    fn scoped_state_requires_permission_and_isolated_per_instance() {
        let runtime = WasmiPluginRuntime::new();

        let mut denied = runtime.instantiate(manifest(), STATE_PLUGIN).unwrap();
        assert_eq!(denied.call_i32("on_load").unwrap(), 0);
        assert_eq!(denied.scoped_state(7), None);

        let mut allowed_manifest = manifest();
        allowed_manifest
            .permissions
            .push(PluginPermission::ScopedStorage);

        let mut first = runtime
            .instantiate(allowed_manifest.clone(), STATE_PLUGIN)
            .unwrap();
        let second = runtime.instantiate(allowed_manifest, STATE_PLUGIN).unwrap();

        assert_eq!(first.call_i32("on_load").unwrap(), 42);
        assert_eq!(first.scoped_state(7), Some(42));
        assert_eq!(second.scoped_state(7), None);
    }

    #[test]
    fn fuel_stops_non_terminating_plugin() {
        let runtime = WasmiPluginRuntime::new();
        let mut plugin = runtime
            .instantiate_with_fuel(manifest(), INFINITE_PLUGIN, 100)
            .unwrap();

        assert!(plugin.call_unit("run").is_err());
        assert_eq!(plugin.remaining_fuel().unwrap(), 0);
    }

    #[test]
    fn fuel_budget_resets_for_each_call() {
        const FUEL_PER_CALL: u64 = 10_000;

        let runtime = WasmiPluginRuntime::new();
        let mut plugin = runtime
            .instantiate_with_fuel(manifest(), CAPABILITY_PLUGIN, FUEL_PER_CALL)
            .unwrap();

        assert_eq!(plugin.call_i32("on_load").unwrap(), HOST_DENY);
        let after_first = plugin.remaining_fuel().unwrap();
        assert!(after_first < FUEL_PER_CALL);

        assert_eq!(plugin.call_i32("on_load").unwrap(), HOST_DENY);
        assert_eq!(plugin.remaining_fuel().unwrap(), after_first);
    }

    #[test]
    fn scoped_host_state_has_bounded_entry_count() {
        let mut allowed_manifest = manifest();
        allowed_manifest
            .permissions
            .push(PluginPermission::ScopedStorage);
        let mut state = PluginHostState::new(allowed_manifest);

        for index in 0..DEFAULT_PLUGIN_STATE_ENTRIES {
            assert!(state.put_scoped_state(i32::try_from(index).unwrap(), 1));
        }
        assert_eq!(state.state.len(), DEFAULT_PLUGIN_STATE_ENTRIES);
        assert!(!state.put_scoped_state(i32::try_from(DEFAULT_PLUGIN_STATE_ENTRIES).unwrap(), 1));

        assert!(state.put_scoped_state(0, 2));
        assert_eq!(state.state.len(), DEFAULT_PLUGIN_STATE_ENTRIES);
        assert_eq!(state.state.get(&0), Some(&2));
    }

    #[test]
    fn memory_limit_rejects_large_initial_memory() {
        let runtime = WasmiPluginRuntime::new();
        assert!(runtime
            .instantiate(manifest(), OVERSIZED_MEMORY_PLUGIN)
            .is_err());
    }

    #[test]
    fn memory_limit_rejects_runtime_growth_past_limit() {
        let runtime = WasmiPluginRuntime::new();
        let mut plugin = runtime.instantiate(manifest(), GROW_MEMORY_PLUGIN).unwrap();

        assert!(plugin.call_i32("grow").is_err());
    }

    #[test]
    fn host_call_budget_denies_calls_after_quota() {
        let runtime = WasmiPluginRuntime::new();
        let mut manifest = manifest();
        manifest
            .permissions
            .push(PluginPermission::EvaluateCapability(
                Capability::MemberRemove,
            ));

        let mut plugin = runtime
            .instantiate_with_budget(manifest, HOST_CALL_BURST_PLUGIN, 10_000, 2)
            .unwrap();

        assert_eq!(plugin.call_i32("run").unwrap(), HOST_DENY);
        assert_eq!(plugin.remaining_host_calls(), 0);
    }

    #[test]
    fn trapped_plugin_rolls_back_scoped_state_changes() {
        let runtime = WasmiPluginRuntime::new();
        let mut manifest = manifest();
        manifest.permissions.push(PluginPermission::ScopedStorage);

        let mut plugin = runtime
            .instantiate(manifest, STATE_THEN_TRAP_PLUGIN)
            .unwrap();

        assert!(plugin.call_unit("run").is_err());
        assert_eq!(plugin.scoped_state(7), None);
    }

    #[test]
    fn undeclared_private_key_and_wasi_imports_are_rejected() {
        let runtime = WasmiPluginRuntime::new();
        assert!(runtime
            .instantiate(manifest(), ROOT_KEY_IMPORT_PLUGIN)
            .is_err());
        assert!(runtime.instantiate(manifest(), WASI_IMPORT_PLUGIN).is_err());
    }

    #[test]
    fn malformed_module_is_rejected_before_execution() {
        let runtime = WasmiPluginRuntime::new();
        assert!(runtime
            .instantiate(manifest(), &[0x00, 0x61, 0x73])
            .is_err());
    }
}
