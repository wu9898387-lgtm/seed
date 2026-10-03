use std::collections::BTreeMap;

use wasmi::{
    Caller, Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::{
    capability::Capability,
    plugin::{PluginManifest, PluginPermission},
};

pub const DEFAULT_PLUGIN_FUEL: u64 = 50_000;
pub const DEFAULT_PLUGIN_MEMORY_BYTES: usize = 64 * 1024;

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
    limits: StoreLimits,
}

impl PluginHostState {
    fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            state: BTreeMap::new(),
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
        self.instantiate_with_fuel(manifest, wasm, DEFAULT_PLUGIN_FUEL)
    }

    pub fn instantiate_with_fuel(
        &self,
        manifest: PluginManifest,
        wasm: &[u8],
        fuel: u64,
    ) -> Result<WasmiPluginInstance, wasmi::Error> {
        let module = Module::new(&self.engine, wasm)?;
        let mut store = Store::new(&self.engine, PluginHostState::new(manifest));
        store.limiter(|state| &mut state.limits);
        store.set_fuel(fuel)?;

        let mut linker = <Linker<PluginHostState>>::new(&self.engine);
        linker.func_wrap(
            "seed",
            "capability_allowed",
            |caller: Caller<'_, PluginHostState>, code: i32| -> i32 {
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
                if !caller.data().scoped_storage_allowed() {
                    return HOST_DENY;
                }
                caller.data_mut().state.insert(key, value);
                HOST_ALLOW
            },
        )?;
        linker.func_wrap(
            "seed",
            "state_get",
            |caller: Caller<'_, PluginHostState>, key: i32| -> i32 {
                if !caller.data().scoped_storage_allowed() {
                    return 0;
                }
                caller.data().state.get(&key).copied().unwrap_or(0)
            },
        )?;

        let instance = linker.instantiate_and_start(&mut store, &module)?;
        Ok(WasmiPluginInstance { store, instance })
    }
}

pub struct WasmiPluginInstance {
    store: Store<PluginHostState>,
    instance: Instance,
}

impl WasmiPluginInstance {
    pub fn call_i32(&mut self, export: &str) -> Result<i32, wasmi::Error> {
        self.instance
            .get_typed_func::<(), i32>(&self.store, export)?
            .call(&mut self.store, ())
    }

    pub fn call_unit(&mut self, export: &str) -> Result<(), wasmi::Error> {
        self.instance
            .get_typed_func::<(), ()>(&self.store, export)?
            .call(&mut self.store, ())
    }

    pub fn remaining_fuel(&self) -> Result<u64, wasmi::Error> {
        self.store.get_fuel()
    }

    pub fn scoped_state(&self, key: i32) -> Option<i32> {
        self.store.data().state.get(&key).copied()
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
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0a, 0x02, 0x60, 0x01, 0x7f,
        0x01, 0x7f, 0x60, 0x00, 0x01, 0x7f, 0x02, 0x1b, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64,
        0x12, 0x63, 0x61, 0x70, 0x61, 0x62, 0x69, 0x6c, 0x69, 0x74, 0x79, 0x5f, 0x61, 0x6c,
        0x6c, 0x6f, 0x77, 0x65, 0x64, 0x00, 0x00, 0x03, 0x02, 0x01, 0x01, 0x07, 0x0b, 0x01,
        0x07, 0x6f, 0x6e, 0x5f, 0x6c, 0x6f, 0x61, 0x64, 0x00, 0x01, 0x0a, 0x08, 0x01, 0x06,
        0x00, 0x41, 0x01, 0x10, 0x00, 0x0b,
    ];

    // Imports seed.state_put(i32, i32)->i32 and seed.state_get(i32)->i32.
    // on_load writes 42 under key 7 and returns the read value.
    const STATE_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x10, 0x03, 0x60, 0x02, 0x7f,
        0x7f, 0x01, 0x7f, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x01, 0x7f, 0x02, 0x23,
        0x02, 0x04, 0x73, 0x65, 0x65, 0x64, 0x09, 0x73, 0x74, 0x61, 0x74, 0x65, 0x5f, 0x70,
        0x75, 0x74, 0x00, 0x00, 0x04, 0x73, 0x65, 0x65, 0x64, 0x09, 0x73, 0x74, 0x61, 0x74,
        0x65, 0x5f, 0x67, 0x65, 0x74, 0x00, 0x01, 0x03, 0x02, 0x01, 0x02, 0x07, 0x0b, 0x01,
        0x07, 0x6f, 0x6e, 0x5f, 0x6c, 0x6f, 0x61, 0x64, 0x00, 0x02, 0x0a, 0x0f, 0x01, 0x0d,
        0x00, 0x41, 0x07, 0x41, 0x2a, 0x10, 0x00, 0x1a, 0x41, 0x07, 0x10, 0x01, 0x0b,
    ];

    // (func (export "run") (loop br 0))
    const INFINITE_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
        0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x00, 0x0a,
        0x09, 0x01, 0x07, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x0b,
    ];

    // (memory 2) => 128 KiB initial memory, above the 64 KiB host limit.
    const OVERSIZED_MEMORY_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, 0x02,
    ];

    // Attempts to import a private-key host API which Seed never links.
    const ROOT_KEY_IMPORT_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
        0x7f, 0x02, 0x19, 0x01, 0x04, 0x73, 0x65, 0x65, 0x64, 0x10, 0x72, 0x6f, 0x6f, 0x74,
        0x5f, 0x70, 0x72, 0x69, 0x76, 0x61, 0x74, 0x65, 0x5f, 0x6b, 0x65, 0x79, 0x00, 0x00,
    ];

    // Attempts to import a WASI network-like API. No WASI namespace is linked.
    const WASI_IMPORT_PLUGIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
        0x02, 0x24, 0x01, 0x16, 0x77, 0x61, 0x73, 0x69, 0x5f, 0x73, 0x6e, 0x61, 0x70, 0x73,
        0x68, 0x6f, 0x74, 0x5f, 0x70, 0x72, 0x65, 0x76, 0x69, 0x65, 0x77, 0x31, 0x09, 0x73,
        0x6f, 0x63, 0x6b, 0x5f, 0x6f, 0x70, 0x65, 0x6e, 0x00, 0x00,
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
            .push(PluginPermission::EvaluateCapability(Capability::MemberRemove));

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
    fn memory_limit_rejects_large_initial_memory() {
        let runtime = WasmiPluginRuntime::new();
        assert!(runtime
            .instantiate(manifest(), OVERSIZED_MEMORY_PLUGIN)
            .is_err());
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
        assert!(runtime.instantiate(manifest(), &[0x00, 0x61, 0x73]).is_err());
    }
}
