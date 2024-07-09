#![feature(asm_goto)]
use jtable::*;

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("trace"));
    define_static_key_true!(TRUE_TEST);
    define_static_key_false!(FALSE_TEST);

    static_branch_likely!(TRUE_TEST);
    static_branch_likely!(FALSE_TEST);
    const XX: usize = 0;
    static_branch_likely!(XX);
}
