use wasmi::{Config, Engine, Linker, Module, Store};

const ADD_ONE_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic + version
    0x01, 0x06, 0x01, 0x60, 0x01, 0x7f, 0x01, 0x7f, // type: (i32) -> i32
    0x03, 0x02, 0x01, 0x00, // function section
    0x07, 0x0b, 0x01, 0x07, b'a', b'd', b'd', b'_', b'o', b'n', b'e', 0x00, 0x00, // export
    0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x41, 0x01, 0x6a, 0x0b, // code
];

fn run_plugin(input: i32) -> Result<(i32, u64), wasmi::Error> {
    let mut config = Config::default();
    config.consume_fuel(true).allow_start_fn(false);

    let engine = Engine::new(&config);
    let module = Module::new(&engine, ADD_ONE_WASM)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;

    let linker = <Linker<()>>::new(&engine);
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    let add_one = instance.get_typed_func::<i32, i32>(&store, "add_one")?;
    let result = add_one.call(&mut store, input)?;
    let remaining_fuel = store.get_fuel()?;

    Ok((result, remaining_fuel))
}

fn main() -> Result<(), wasmi::Error> {
    let (result, remaining_fuel) = run_plugin(41)?;
    assert_eq!(result, 42);
    assert!(remaining_fuel < 10_000);

    println!("Seed Phase 0 WASM plugin runtime spike");
    println!("result={result}");
    println!("fuel_used={}", 10_000 - remaining_fuel);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run_plugin;

    #[test]
    fn validated_wasm_executes_inside_fuel_budget() {
        let (result, remaining_fuel) = run_plugin(41).unwrap();
        assert_eq!(result, 42);
        assert!(remaining_fuel < 10_000);
    }
}
