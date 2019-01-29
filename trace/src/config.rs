use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

use crate::id_generator::{IDGenerator, DEFAULT_ID_GENERATOR};
use crate::sampling::{Sampler, DEFAULT_SAMPLER};

/// Config represents the global tracing configuration.
#[derive(Clone)]
pub struct Config {
    /// default_sampler is the default sampler used when creating new spans.
    pub default_sampler: Sampler,

    /// id_generator is for internal use only.
    pub id_generator: Arc<dyn IDGenerator + Send + Sync>,
}

lazy_static! {
    /// Global tracing configuration.
    static ref CONFIG: RwLock<Config> = RwLock::new(Config {
        default_sampler: DEFAULT_SAMPLER.clone(),
        id_generator: DEFAULT_ID_GENERATOR.clone(),
    });
}

pub fn set_global_default_sampler(sampler: &Sampler) {
    let mut c = CONFIG.write().unwrap();
    c.default_sampler = sampler.clone();
}

pub fn set_global_id_generator(id_generator: &Arc<dyn IDGenerator + Send + Sync>) {
    let mut c = CONFIG.write().unwrap();
    c.id_generator = Arc::clone(id_generator);
}

/// load_config retrieves a copy of the global tracing configuration.
pub fn load_config() -> Config {
    let c = CONFIG.read().unwrap();
    c.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_zero_config() {
        let config = {
            let config_lock = CONFIG.read().unwrap();
            config_lock.clone()
        };
        set_global_id_generator(&config.id_generator);
        set_global_default_sampler(&config.default_sampler);
        let current_cfg = CONFIG.read().unwrap();

        assert!(Sampler::ptr_eq(
            &current_cfg.default_sampler,
            &config.default_sampler
        ));
        assert!(Arc::ptr_eq(&current_cfg.id_generator, &config.id_generator));
    }
}
