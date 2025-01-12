//! This module is called mod.rs so when running tests, Cargo knows not to report this
//! as a test module, but treat it as module full of test utilities
//!
//! By simply importing or declaring this module, /tests test programs will have logging inited

use lazy_static::lazy_static;

lazy_static! {
    static ref _LOGGER_INIT: bool = {
        env_logger::init();
        true
    };
}
