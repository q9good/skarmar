#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runtime {
    Local,
    Cloudflare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    Sqlite,
    DurableObject,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub storage: Storage,
    pub demo: bool,
    pub space: String,
}

impl Config {
    /// Runtime capabilities come from the executable, not from an untrusted request.
    /// A wrong configuration is an error: never fall back to an empty database.
    pub fn load(runtime: Runtime, lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let default = match runtime {
            Runtime::Local => "sqlite",
            Runtime::Cloudflare => "durable_object",
        };
        let storage = match lookup("SKARMA_STORAGE").as_deref().unwrap_or(default) {
            "sqlite" => Storage::Sqlite,
            "durable_object" => Storage::DurableObject,
            _ => return Err("SKARMA_STORAGE 必须是 sqlite 或 durable_object".into()),
        };
        if !matches!(
            (runtime, storage),
            (Runtime::Local, Storage::Sqlite) | (Runtime::Cloudflare, Storage::DurableObject)
        ) {
            return Err("存储与运行入口不匹配：sqlite 使用本地 API，durable_object 使用 Worker（可在本机运行 wrangler dev）".into());
        }
        let demo = match lookup("SKARMA_DEMO").as_deref().unwrap_or("0") {
            "0" => false,
            "1" => true,
            _ => return Err("SKARMA_DEMO 必须是 0 或 1".into()),
        };
        let space = lookup("SKARMA_SPACE").unwrap_or_else(|| "default".into());
        if space.trim().is_empty() || space.len() > 128 {
            return Err("SKARMA_SPACE 不能为空或超过 128 字节".into());
        }
        Ok(Self {
            storage,
            demo,
            space,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_storage_does_not_silently_use_defaults() {
        assert!(
            Config::load(Runtime::Local, |key| (key == "SKARMA_STORAGE")
                .then(|| "d1".into()))
            .is_err()
        );
        assert!(
            Config::load(Runtime::Local, |key| (key == "SKARMA_STORAGE")
                .then(|| "durable_object".into()))
            .is_err()
        );
        assert!(
            Config::load(Runtime::Cloudflare, |key| (key == "SKARMA_STORAGE")
                .then(|| "sqlite".into()))
            .is_err()
        );
    }
    #[test]
    fn defaults_and_sample_data_are_explicit() {
        let local = Config::load(Runtime::Local, |_| None).unwrap();
        let cf = Config::load(Runtime::Cloudflare, |_| None).unwrap();
        assert_eq!(local.storage, Storage::Sqlite);
        assert_eq!(cf.storage, Storage::DurableObject);
        assert!(!local.demo && !cf.demo);
        assert!(
            Config::load(Runtime::Local, |key| (key == "SKARMA_DEMO")
                .then(|| "true".into()))
            .is_err()
        );
        assert!(
            Config::load(Runtime::Cloudflare, |key| (key == "SKARMA_SPACE")
                .then(String::new))
            .is_err()
        );
    }
}
