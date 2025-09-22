# Delta 1 — Modules

> Overzicht van alle kernmodules binnen de modulaire Rust-monoliet en de
> C-ABI die via de PHP-interface beschikbaar is. De beschrijving volgt de
> huidige implementatie in `rust-core/` zodat ontwerpdocumentatie en code
> in sync blijven.

---

## Inhoud

* [`common`](#common)
* [`data`](#data)
* [`training`](#training)
* [`inference`](#inference)
* [`evaluation`](#evaluation)
* [`api::ffi`](#apiffi)
* [Event- & logvelden](#event--logvelden)
* [Foutcodes](#foutcodes)
* [Configuratie](#configuratie)
* [Testmatrix](#testmatrix)
* [Snelle referentie: module-grenzen](#snelle-referentie-module-grenzen)
* [Beveiliging & privacy](#beveiliging--privacy)

---

## `common`

**Doel**

Gedeelde hulpprogramma’s (configuratie, errors, hashing, JSON-tools, logging)
waar alle domeinen op leunen.

**Submodules**

* `config` — `AppCfg::load()` leest `DELTA1_*`-omgevingsvariabelen.
* `error` — `DeltaError` + stabiele `DeltaCode` (0..5) voor FFI.
* `ids` — deterministische `SimpleHash` helpers (32/64-bit & hex).
* `json` — mini-helpers voor escaping, key lookup en string arrays.
* `log` — `log_json(level, module, event, code, dur_ms)` → JSON logging.
* `time` — monotone klok (`now_ms`).
* `buf` — eenvoudige, herbruikbare buffer voor IO-intensieve paden.

**Kern-API (extract)**

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

pub struct AppCfg { pub data_root: String, pub region: String, pub log_level: u8 }
pub fn load_cfg() -> AppCfg; // wrapper rond AppCfg::load()
```

---

## `data`

**Doel**

Veilige data-acquisitie en documentatie van datasets.

**Structuur**

`domain.rs` (types & traits) • `service.rs` (ingest + datasheet export) •
`repo_fs.rs` (placeholder voor bestandsopslag).

**Belangrijkste types**

* `DatasetId` — stringwrapper (`ds-<hash>`).
* `Dataset` — metadata (`schema`, `created_ms`, `rows`).
* `DataRepo` — trait voor persistente opslag (nog niet ingevuld).

**Publieke service-functies**

```rust
pub fn ingest_file(path: &str, schema_json: &str) -> Result<DatasetId, DeltaError>;
pub fn export_datasheet(dataset_id: &DatasetId) -> Result<String, DeltaError>;
```

**FFI-contract**

```c
int  delta1_data_ingest(const char* filepath, char** out_dataset_id);
const char* delta1_export_datasheet(const char* dataset_id);
```

`delta1_data_ingest` retourneert een `DeltaCode` (`0` bij succes) en schrijft de
nieuwe dataset-id naar `out_dataset_id` (caller → `delta1_free_str`).

---

## `training`

**Doel**

Modellen trainen, versies beheren en governance-metadata (DP/fairness) vastleggen.

**Belangrijkste types**

* `ModelId` — deterministisch op basis van dataset, config en modelsoort.
* `VersionName` — wrapper rond een string (`v<timestamp>`).
* `ModelVersion` — bevat id, versie, `ModelKind`, artefact-pad en metadata.
* `TrainConfig` — parseert JSON (`model_kind`, `dp`, `fairness`).
* `ModelRepo`, `Trainer` — traits voor persistente opslag / trainers.

**Servicefuncties**

```rust
pub fn train(dataset: DatasetId, cfg_json: &str) -> Result<ModelVersion, DeltaError>;
pub fn load_model(id: &ModelId, version: Option<&VersionName>)
    -> Result<ModelVersion, DeltaError>;
pub fn export_model_card(id: &ModelId) -> Result<String, DeltaError>;
```

Tijdens `train` worden DP-bounds (`epsilon ≤ 3`, `delta ≤ 1e-5`, `clip > 0`,
`noise_multiplier > 0`) én fairness-drempels (`ΔTPR/FPR/PPV`) enforced. Alle
versies leven voorlopig in een in-memory registry (mutex-beveiligd).

**FFI-contract**

```c
int  delta1_train(const char* dataset_id,
                  const char* train_cfg_json,
                  char** out_model_id);
int  delta1_load_model(const char* model_id, const char* version); // versie optioneel
const char* delta1_export_model_card(const char* model_id);
```

`delta1_train`/`delta1_load_model` geven `DeltaCode` terug; `delta1_export_model_card`
levert JSON (`model_id`, `version`, DP/fairness) dat door PHP moet worden vrijgegeven.

---

## `inference`

**Doel**

Synchron inferentie, routering tussen tabulaire en tekst-engines, consent-checks
en WhyLog-export.

**Belangrijkste elementen**

* Router (`SSMRouter`) bepaalt `RouteTarget::Tabular|Text` op basis van payload.
* `AllowAllConsent` is de huidige store (TODO: echte opslagkoppeling).
* `EngineRegistry` bevat `TabularEngine` en `TextEngine` (deterministische mocks).
* `Prediction` — JSON-antwoord + latency, confidence en WhyLog-info.

**Publieke service-functies**

```rust
pub fn register_active_model(model: ModelVersion);
pub fn infer_with_ctx(purpose_id: &str, subject_id: &str, input_json: &str)
    -> Result<Prediction, DeltaError>;
pub fn infer_with_model(model_id: &ModelId, version: Option<&VersionName>,
    purpose_id: &str, subject_id: &str, input_json: &str)
    -> Result<Prediction, DeltaError>;
```

Consent wordt nu permissief behandeld, maar het contract is aanwezig (`ConsentStore`).
Bij tekstfouten valt de router terug naar tabular. Elke respons krijgt een
WhyLog-hash (`SimpleHash::finish_hex64`).

**FFI-contract**

```c
int         delta1_load_model(const char* model_id, const char* version); // reuse training
const char* delta1_infer_with_ctx(const char* purpose_id,
                                  const char* subject_id,
                                  const char* input_json);
```

De JSON-uitvoer bevat o.a. `model_id`, `version`, `route`, `confidence` en
`whylog_hash`. Callers moeten altijd `delta1_free_str` gebruiken.

---

## `evaluation`

**Doel**

Evaluatie- en drift-API. De huidige implementatie bevat scaffolding:

```rust
pub fn evaluate(model: &ModelVersion) -> Result<EvalSuite, DeltaError>; // retourneert lege kaart
pub fn drift(model: &ModelVersion) -> Result<DriftStats, DeltaError>;    // Err(not_implemented)
```

`EvalSuite` houdt een kopie van het model plus een placeholder voor de metrics.
Drift-detectie wordt nog niet uitgevoerd.

---

## `api::ffi`

**Doel**

Enige grens met de buitenwereld. Verzorgt pointer-checks, `CString`-beheer en het
mappen van `DeltaError` → `DeltaCode` (als `i32`).

**Exporttabel**

```c
const char* delta1_api_version(void); // "1.0.0" (pointer naar interne CString)
int         delta1_data_ingest(const char* filepath, char** out_dataset_id);
int         delta1_train(const char* dataset_id,
                         const char* train_cfg_json,
                         char** out_model_id);
int         delta1_load_model(const char* model_id, const char* version_opt);
const char* delta1_infer_with_ctx(const char* purpose_id,
                                  const char* subject_id,
                                  const char* input_json);
const char* delta1_export_model_card(const char* model_id);
const char* delta1_export_datasheet(const char* dataset_id);
void        delta1_free_str(const char* ptr);
```

`delta1_api_version` retourneert een pointer naar een intern beheerde string en
hoeft niet vrijgegeven te worden. Alle andere `const char*`-resultaten moeten via
`delta1_free_str` worden opgeruimd.

---

## Event- & logvelden

**Log (JSON)**

* `ts`, `level`, `mod`, `event`, `code`, `dur_ms`

**Metrics**

* `infer_latency_ms`, `infer_qps`, `error_ratio`, `drift_psi`, `train_dur_ms`

---

## Foutcodes

| Code | Naam         | Betekenis kort              |
| ---: | ------------ | --------------------------- |
|    0 | Ok           | Succes                      |
|    1 | NoConsent    | Consent geweigerd/afwezig   |
|    2 | PolicyDenied | DP/fairness/policy faalde   |
|    3 | ModelMissing | Geen actief model of versie |
|    4 | InvalidInput | Validatie faalde / null ptr |
|    5 | Internal     | Onverwachte fout / TODO     |

---

## Configuratie

`AppCfg::load()` leest momenteel drie kernwaarden:

| Sleutel            | Voorbeeld     | Omschrijving                         |
| ------------------ | ------------- | ------------------------------------ |
| `DELTA1_DATA_ROOT` | `/var/delta1` | Basis-pad voor datasets/modellen     |
| `DELTA1_REGION`    | `eu-west`     | Regioreferentie voor governance      |
| `DELTA1_LOG_LEVEL` | `1`           | Loggingniveau (`0=error` .. `3=debug`)|

Policies, DP-drempels en routerregels zitten in code/JSON-config (nog geen env-keys).

---

## Testmatrix

| Laag               | Soort    | Wat                                                     |
| ------------------ | -------- | ------------------------------------------------------- |
| Rust/common        | Unit     | `DeltaCode`, hashing, JSON-helpers                      |
| Rust/data          | Unit     | Ingest hashing, datasheet-export                        |
| Rust/training      | Unit     | DP/fairness-gates, modelkaart                           |
| Rust/inference     | Unit     | Router, fallback, WhyLog-hash                           |
| Rust/api::ffi      | Contract | Null-checks, `DeltaCode`, `delta1_free_str` ownership   |
| PHP-interface      | Wrapper  | Mapping naar HTTP, memory management                    |
| Integratie (toekomst) | E2E   | ingest → train → load_model → infer_with_ctx → export   |

---

## Snelle referentie: module-grenzen

| Module       | Roept aan                                | Exporteert naar buiten            |
| ------------ | ---------------------------------------- | --------------------------------- |
| `data`       | `common::{error,ids,time,json}`          | via `api::ffi::delta1_data_ingest` |
| `training`   | `common`, `data::domain::DatasetId`      | via `api::ffi::{train,load_model,export_model_card}` |
| `inference`  | `common`, `training`                     | via `api::ffi::delta1_infer_with_ctx` |
| `evaluation` | `common`, `training`                     | (intern/rapportage, nog stub)     |
| `api::ffi`   | alle domeinen                           | C-ABI richting PHP                |

---

## Beveiliging & privacy

* `data`: hash-gebaseerde identifiers, toekomstige DataRepo voor EU-resident opslag.
* `training`: DP- en fairness-gates verplicht in `train`.
* `inference`: consentcontract aanwezig, WhyLog-hash in elke respons, fallback bij engine-fouten.
* `api::ffi`: `DeltaCode`-map, null-pointer checks, verplicht `delta1_free_str` voor geheugenbeheer.

