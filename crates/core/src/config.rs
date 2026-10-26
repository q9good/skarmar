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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataModel {
    Legacy,
    PrdV1,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub storage: Storage,
    pub model: DataModel,
    pub initialize_empty_prd: bool,
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
        let model = match lookup("SKARMA_MODEL").as_deref().unwrap_or("legacy") {
            "legacy" => DataModel::Legacy,
            "prd_v1" => DataModel::PrdV1,
            _ => return Err("SKARMA_MODEL 必须是 legacy 或 prd_v1".into()),
        };
        if model == DataModel::PrdV1 && demo {
            return Err("新模型使用独立空库和真实登录，不加载旧版示例数据".into());
        }
        let initialize_empty_prd = match lookup("SKARMA_PRD_BASELINE").as_deref() {
            None => false,
            Some("empty_preserve_legacy_v1") if model == DataModel::PrdV1 && !demo => true,
            _ => return Err("SKARMA_PRD_BASELINE 仅支持新模型的 empty_preserve_legacy_v1".into()),
        };
        let space = lookup("SKARMA_SPACE").unwrap_or_else(|| "default".into());
        if space.trim().is_empty() || space.len() > 128 {
            return Err("SKARMA_SPACE 不能为空或超过 128 字节".into());
        }
        Ok(Self {
            storage,
            model,
            initialize_empty_prd,
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
    fn new_model_is_explicit_and_does_not_load_legacy_demo() {
        assert_eq!(
            Config::load(Runtime::Local, |_| None).unwrap().model,
            DataModel::Legacy
        );
        assert_eq!(
            Config::load(Runtime::Cloudflare, |key| (key == "SKARMA_MODEL")
                .then(|| "prd_v1".into()))
            .unwrap()
            .model,
            DataModel::PrdV1
        );
        assert!(
            Config::load(Runtime::Local, |key| (key == "SKARMA_MODEL")
                .then(|| "unknown".into()))
            .is_err()
        );
        assert!(
            Config::load(Runtime::Local, |key| match key {
                "SKARMA_MODEL" => Some("prd_v1".into()),
                "SKARMA_DEMO" => Some("1".into()),
                _ => None,
            })
            .is_err()
        );
    }
    #[test]
    fn empty_baseline_is_an_explicit_prd_only_setting() {
        assert!(
            !Config::load(Runtime::Local, |_| None)
                .unwrap()
                .initialize_empty_prd
        );
        assert!(
            Config::load(Runtime::Local, |key| match key {
                "SKARMA_MODEL" => Some("prd_v1".into()),
                "SKARMA_PRD_BASELINE" => Some("empty_preserve_legacy_v1".into()),
                _ => None,
            })
            .unwrap()
            .initialize_empty_prd
        );
        for value in ["empty_preserve_legacy_v1", "delete", ""] {
            assert!(
                Config::load(Runtime::Cloudflare, |key| (key == "SKARMA_PRD_BASELINE")
                    .then(|| value.into()))
                .is_err()
            );
        }
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
