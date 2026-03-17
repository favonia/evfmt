#![no_main]

use evfmt::scanner::scan_crosscheck;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let (legacy, state_machine) = scan_crosscheck(data);
    assert_eq!(state_machine, legacy, "scanner mismatch for input {data:?}");
});
