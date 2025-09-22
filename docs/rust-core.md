# Delta 1 — Rust Core Architectuur

> Samenvatting van de actuele stand van de crate `rust-core/`. Deze beschrijving
> volgt de concrete code zodat ontwerpbeslissingen en documentatie synchroon
> blijven. Alle voorbeelden zijn dependency-vrij (alleen `std`).

---

## 1. Scope & doelstellingen

* **Scope**: `rust-core/` (`cdylib`) met domeinen `data`, `training`, `inference`,
  `evaluation`, ondersteund door `common` en de `api::ffi`-laag.
* **Doelen**: determinisme, memory-safety, stabiele C-ABI, auditbare governance,
  zo min mogelijk externe dependencies.

---

## 2. Moduleboom

```
rust-core/
└── src/
    ├── lib.rs                 # orchestrator, re-exports (`core_*` helpers)
    ├── api/
    │   ├── mod.rs
    │   └── ffi.rs             # #[no_mangle] extern "C" functies
    ├── common/
    │   ├── buf.rs             # eenvoudige bufferhulpen
    │   ├── config.rs          # AppCfg::load() (ENV)
    │   ├── error.rs           # DeltaError + DeltaCode (0..5)
    │   ├── ids.rs             # SimpleHash helpers
    │   ├── json.rs            # minimale JSON utils
    │   ├── log.rs             # log_json() → JSONL
    │   └── time.rs            # monotone klok
    ├── data/
    │   ├── mod.rs
    │   ├── domain.rs          # DatasetId, Dataset, DataRepo
    │   ├── service.rs         # ingest_file(), export_datasheet()
    │   └── repo_fs.rs         # scaffolding voor FS-opslag
    ├── training/
    │   ├── mod.rs
    │   ├── domain.rs          # ModelId, TrainConfig, metadata
    │   ├── service.rs         # train(), load_model(), export_model_card()
    │   └── repo_fs.rs         # artefact IO (placeholder)
    ├── inference/
    │   ├── mod.rs
    │   ├── domain.rs          # routing, consent, Prediction
    │   ├── service.rs         # register_active_model(), infer_with_ctx()
    │   └── workers.rs         # threadpool (std::thread + mpsc)
    └── evaluation/
        ├── mod.rs
        ├── domain.rs          # EvalSuite, DriftStats
        └── service.rs         # evaluate(), drift() (stub)
```

---

## 3. Cross-module contracten

Alle domeinen delen compacte types en traits:

```rust
// data/domain.rs
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DatasetId(String); // stringwrapper (ds-<hash>)

pub struct Dataset {
    pub id: DatasetId,
    pub schema: Schema,
    pub created_ms: u128,
    pub rows: u64,
}

pub trait DataRepo {
    fn put_dataset(&self, dataset: &Dataset) -> DeltaResult<()>;
    fn get_dataset(&self, id: DatasetId) -> DeltaResult<Dataset>;
}

// training/domain.rs
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ModelId(String);
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct VersionName(String);

pub struct ModelVersion {
    pub id: ModelId,
    pub version: VersionName,
    pub kind: ModelKind,
    pub artefact_path: String,
    pub metadata: ModelMetadata,
}

pub trait ModelRepo {
    fn put_model(&self, model: &ModelVersion) -> DeltaResult<()>;
    fn get_model(&self, id: &ModelId, version: &VersionName)
        -> DeltaResult<ModelVersion>;
}

// inference/domain.rs
pub struct Prediction {
    pub json: String,
    pub latency_ms: u32,
    pub confidence: f32,
    pub whylog: WhyLog,
}

pub trait InferEngine {
    fn kind(&self) -> RouteTarget;
    fn infer(&self, model: &ModelVersion, input_json: &str)
        -> DeltaResult<EngineResponse>;
}
```

---

## 4. Foutafhandeling

`common/error.rs` definieert de stabiele errorcodes die over de C-ABI gaan:

```rust
#[repr(u32)]
pub enum DeltaCode {
    Ok = 0,
    NoConsent = 1,
    PolicyDenied = 2,
    ModelMissing = 3,
    InvalidInput = 4,
    Internal = 5,
}

pub struct DeltaError {
    pub code: DeltaCode,
    pub msg: &'static str,
}
```

Helpers zoals `DeltaError::policy_denied("dp_epsilon_exceeded")` of
`DeltaError::invalid("ffi_null")` worden consequent gebruikt in alle services.
`api::ffi` vertaalt errors naar `i32` zodat PHP eenvoudig kan inspecteren.

---

## 5. Configuratie

`common/config.rs` laadt omgevingsvariabelen zonder externe parser:

```rust
impl AppCfg {
    pub fn load() -> Self {
        fn env_or(key: &str, default: &str) -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        }
        Self {
            data_root: env_or("DELTA1_DATA_ROOT", "./data"),
            region: env_or("DELTA1_REGION", "eu"),
            log_level: env_or("DELTA1_LOG_LEVEL", "1").parse().unwrap_or(1),
        }
    }
}
```

`lib.rs` biedt een `load_cfg()`-wrapper voor achterwaartse compatibiliteit met de
oude documentatie.

---

## 6. Logging & observability

`common/log.rs` schrijft compacte JSON-regels naar stdout:

```rust
pub fn log_json(level: &str, module: &str, event: &str, code: u32, dur_ms: u128) {
    let ts = crate::common::time::now_ms();
    println!(
        "{{\"ts\":{ts},\"level\":\"{level}\",\"mod\":\"{module}\",\"ev\":\"{event}\",\"code\":{code},\"dur_ms\":{dur_ms}}}"
    );
}
```

Metrics zoals `infer_latency_ms` of `train_dur_ms` worden later toegevoegd; de
logstructuur is alvast stabiel.

---

## 7. Concurrency-model

Alle concurrency bouwt op `std`:

```rust
// inference/workers.rs
pub struct Pool {
    tx: mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl Pool {
    pub fn new(size: usize) -> Self { /* spawns std::thread workers */ }
    pub fn submit<F>(&self, job: F)
    where F: FnOnce() + Send + 'static,
    {
        let _ = self.tx.send(Box::new(job));
    }
}
```

`inference::service` gebruikt momenteel synchrone paden; de worker-pool staat klaar
voor CPU-intensieve taken wanneer echte modellen worden aangesloten.

---

## 8. Data-pad

`data::service::ingest_file` leest bestanden streaming met `BufRead::read_line`:

```rust
pub fn ingest_file(path: &str, schema_json: &str) -> DeltaResult<DatasetId> {
    let file = File::open(Path::new(path)).map_err(|_| DeltaError::io())?;
    let mut reader = BufReader::new(file);
    let mut hasher = SimpleHash::new();
    let mut line = String::new();
    let mut rows = 0u64;

    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|_| DeltaError::io())? == 0 {
            break;
        }
        hasher.update(line.as_bytes());
        rows += 1;
    }

    let dataset_id = DatasetId::new(format!("ds-{}", hasher.finish_hex()));
    let _dataset = Dataset::new(dataset_id.clone(), schema_json.to_string(),
                                time::now_ms(), rows);
    Ok(dataset_id)
}
```

`export_datasheet` levert alvast een JSON-placeholder met `retention_days`.
Persistente opslag (`DataRepo`) volgt later.

---

## 9. Training-pad

`training::service::train` verwerkt DP- en fairness-gates voordat een model wordt
opgeslagen in een in-memory registry:

```rust
pub fn train(dataset: DatasetId, cfg_json: &str) -> DeltaResult<ModelVersion> {
    let cfg = TrainConfig::parse(cfg_json.to_string())?;
    enforce_dp(&cfg)?;
    enforce_fairness(&cfg)?;

    let model_id = make_model_id(&dataset, cfg_json, cfg.model_kind());
    let version = VersionName::new(format!("v{}", time::now_ms()));
    let artefact_path = format!("models/{}/{}/model.bin", model_id.as_str(), version.as_str());

    let model = ModelVersion { /* metadata inclusief DP/fairness */ };
    registry().lock()?.insert(model.clone());
    Ok(model)
}
```

`export_model_card` projecteert DP- en fairnessmetadata naar JSON zodat PHP deze
kan aanbieden aan auditors. `load_model` haalt de laatste of gevraagde versie op en
wordt door `api::ffi::delta1_load_model` gebruikt om het actieve model te registreren.

---

## 10. Inferentie

`inference::service::infer_with_ctx` bundelt consent, routering en engine-calls:

```rust
pub fn infer_with_ctx(purpose: &str, subject: &str, input_json: &str)
    -> DeltaResult<Prediction>
{
    let model = active_model().ok_or_else(|| DeltaError::model_missing("active_model"))?;
    let context = build_context(purpose, subject, input_json);
    ensure_consent(consent_store(), &context)?; // huidige implementatie: allow-all

    let router_ctx = RouterContext::from_payload(input_json, &context);
    let decision = ensure_compatible(&model, router().route(&router_ctx));

    let start = time::now_ms();
    let response = match engines().infer(decision.target, &model, input_json) {
        Ok(resp) => resp,
        Err(err) if decision.target == RouteTarget::Text => {
            engines().infer(RouteTarget::Tabular, &model, input_json)?
        }
        Err(err) => return Err(err),
    };
    let latency = time::now_ms().saturating_sub(start) as u32;

    let mut body = merge_payload(&response.payload, &model, decision, response.confidence);
    let whylog = build_whylog(&body, &response);
    append_whylog_hash(&mut body, &whylog.hash);

    Ok(Prediction { json: body, latency_ms: latency, confidence: response.confidence, whylog })
}
```

Tekstpaden die falen vallen terug naar tabular (`RouteTarget::Tabular`). Elke
respons bevat `whylog_hash`, `route`, `confidence` en `model_id`.

---

## 11. Evaluatie

`evaluation::service` bevat placeholderfuncties:

```rust
pub fn evaluate(model: &ModelVersion) -> DeltaResult<EvalSuite> {
    Ok(EvalSuite { model: model.clone(), metrics_card: "{}".into() })
}

pub fn drift(model: &ModelVersion) -> DeltaResult<DriftStats> {
    Err(DeltaError::not_implemented("evaluation::service::drift"))
}
```

De structuren zijn aanwezig zodat latere implementaties (metrics, PSI/KS) direct
kunnen inhaken.

---

## 12. FFI-contract

`api::ffi` vormt de stabiele grens voor PHP. Kernfuncties:

```rust
#[no_mangle]
pub extern "C" fn delta1_api_version() -> *const c_char { /* "1.0.0" */ }
#[no_mangle]
pub extern "C" fn delta1_data_ingest(filepath: *const c_char, out_dataset_id: *mut *const c_char) -> i32;
#[no_mangle]
pub extern "C" fn delta1_train(dataset_id: *const c_char,
                                 train_cfg_json: *const c_char,
                                 out_model_id: *mut *const c_char) -> i32;
#[no_mangle]
pub extern "C" fn delta1_load_model(model_id: *const c_char, version: *const c_char) -> i32;
#[no_mangle]
pub extern "C" fn delta1_infer_with_ctx(purpose_id: *const c_char,
                                          subject_id: *const c_char,
                                          input_json: *const c_char) -> *const c_char;
#[no_mangle]
pub extern "C" fn delta1_export_model_card(model_id: *const c_char) -> *const c_char;
#[no_mangle]
pub extern "C" fn delta1_export_datasheet(dataset_id: *const c_char) -> *const c_char;
#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char);
```

* Returnwaarden (`i32`) zijn `DeltaCode`-waarden.
* `assign_out_string()` en `string_to_raw()` zorgen voor veilig geheugenbeheer.
* `delta1_api_version` geeft een pointer naar een intern beheerde `CString`; niet vrijgeven.

---

## 13. Repository-overzicht (`lib.rs`)

`lib.rs` re-exporteert een smalle façade voor interne callers:

```rust
pub use data::service::{export_datasheet, ingest_file as core_data_ingest};
pub use inference::service::{infer_with_ctx as core_infer_with_ctx, register_active_model};
pub use training::service::{export_model_card, load_model as core_load_model, train as core_train};
```

Zo kan de FFI-laag (`api::ffi`) stabiel blijven terwijl implementatiedetails per
module evolueren.

---

## 14. Toekomstige uitbreidingen

* **Persistente repos**: koppel `DataRepo` en `ModelRepo` aan FS/DB zodra
  retentionbeleid is uitgewerkt.
* **Consent store**: vervang `AllowAllConsent` door een echte opslaglaag via FFI.
* **Evaluatie**: implementeer metric- en driftberekeningen, exporteer rapporten via FFI.
* **Observability**: breid `log_json` uit met sampling en metrics.

