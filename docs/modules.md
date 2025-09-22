# Delta 1 — Modules

> Overzicht van alle kernmodules binnen de modulaire Rust-monoliet en hun contracten richting de PHP-interface (FFI). Focus op heldere grenzen, FFI-veilige types en Europese privacy-principes (dataminimalisatie, auditability).

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

---

## `common`

**Doel**
Gedeelde types, errors, config, logging/telemetry, util.

**Submodules**

* `error` — `DeltaError` + mapping naar numerieke `DeltaCode`.
* `config` — strikte TOML/ENV parsing; immutable.
* `telemetry` — structured logging, timers, counters.

**Kern-API (Rust)**

```rust
pub enum DeltaCode { Ok=0, InvalidInput=10, Io=20, NotFound=30, Internal=50 }

pub struct AppCfg { /* immutable config */ }
pub fn load_cfg() -> Result<AppCfg, DeltaError>;

pub fn now_ms() -> u128;
pub fn log_json(module: &str, event: &str, kv: &[(&str, &str)]);
```

---

## `data`

**Doel**
Veilige data-acquisitie, schema-validatie, normalisatie en registratie van datasets.

**Structuur**
`domain/{entity.rs,service.rs,repository.rs}` • `infrastructure/{fs.rs,stream.rs}`

**Entiteiten**

* `Dataset { id:u32, hash:String, schema:Schema, created_at:DateTime }`
* `Schema { json:String }`

**Publieke service-functies (Rust)**

```rust
pub fn ingest_file(path: &str, schema_json: &str) -> Result<u32, DeltaError>;
pub fn dataset_meta(id: u32) -> Result<Dataset, DeltaError>;
```

**FFI-signature (C-ABI)**

```c
unsigned int delta1_data_ingest(const char* path, const char* schema_json); /* >0 ok, 0 fout */
```

**Validatieregels (kort)**

* Pad bestaat, bestand leesbaar.
* Schema verplicht; velden geminimaliseerd (PII vermijden/hasher).

---

## `training`

**Doel**
Bouw, hertrain en versieer modellen; produceer artefact + modelkaart/metrics.

**Entiteiten**

* `ModelVersion { id:u32, version:String, artefact_path:String, metrics:Json }`
* `TrainConfig { hyperparams:Json, seed:u64, notes:Option<String> }`

**Publieke service-functies (Rust)**

```rust
pub fn train(dataset_id: u32, cfg: &str) -> Result<u32, DeltaError>; // return model_id
pub fn model_meta(model_id: u32) -> Result<ModelVersion, DeltaError>;
```

**FFI**

```c
unsigned int delta1_train(unsigned int dataset_id, const char* config_json);
```

**Uitvoer artefact**
`/var/delta1/models/<model_id>-<version>.bin` (EU-residency; at-rest versleuteld).

---

## `inference`

**Doel**
Realtime/batch voorspellingen, thresholds, A/B/shadow-routing, post-checks.

**Entiteiten**

* `PredictionRequest { json:String }`
* `PredictionResponse { json:String, latency_ms:u32, confidence:f32 }`
* `Route { primary:u32, shadow:Option<u32>, ratio:u8 }`

**Publieke service-functies (Rust)**

```rust
pub fn infer(model_id: u32, input_json: &str) -> Result<String, DeltaError>;
pub fn set_route(route: Route) -> Result<(), DeltaError>;
```

**FFI**

```c
const char* delta1_infer(unsigned int model_id, const char* input_json);
void        delta1_free_str(const char* ptr);
```

**Post-checks**

* JSON-schema validatie output.
* Confidence/thresholds; flag voor HITL.

---

## `evaluation`

**Doel**
Offline evaluaties, bias/fairness-checks, canaries, rapportage.

**Entiteiten**

* `EvalSuite { metrics:Json, fairness:Json }`
* `DriftStats { psi:Json, ks:Json }`

**Publieke service-functies (Rust)**

```rust
pub fn evaluate(model_id: u32, dataset_id: u32) -> Result<String, DeltaError>;
pub fn drift(model_id: u32) -> Result<String, DeltaError>;
```

**Rapportage**
Schrijft samenvatting naar `metrics_json`, `bias_json` in `models`.

---

## `api::ffi`

**Doel**
Enige grens met de buitenwereld. C-ABI stabiel, semver via `delta1_api_version()`.

**Exporttabel**

```c
unsigned int delta1_api_version(void);                /* 1 */
unsigned int delta1_data_ingest(const char*, const char*);
unsigned int delta1_train(unsigned int, const char*);
const char*  delta1_infer(unsigned int, const char*);
void         delta1_free_str(const char*);
```

**Implementatiepatroon (Rust)**

```rust
use std::ffi::{CStr, CString};
#[no_mangle] pub extern "C" fn delta1_api_version() -> u32 { 1 }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(path:*const c_char, schema:*const c_char) -> u32 {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let schema = unsafe { CStr::from_ptr(schema) }.to_string_lossy();
    match crate::data::ingest_file(&path, &schema) { Ok(id) => id, Err(_) => 0 }
}

#[no_mangle]
pub extern "C" fn delta1_infer(model_id:u32, input:*const c_char) -> *const c_char {
    let input = unsafe { CStr::from_ptr(input) }.to_string_lossy();
    let out = crate::inference::infer(model_id, &input).unwrap_or_else(|_| "{}".into());
    CString::new(out).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr:*const c_char) {
    if ptr.is_null() { return; }
    unsafe { let _ = CString::from_raw(ptr as *mut c_char); }
}
```

**PHP-wrapper (voorbeeld)**

```php
<?php
$ffi = FFI::cdef('
  unsigned int delta1_api_version(void);
  unsigned int delta1_data_ingest(const char* path, const char* schema_json);
  unsigned int delta1_train(unsigned int dataset_id, const char* config_json);
  const char*  delta1_infer(unsigned int model_id, const char* input_json);
  void         delta1_free_str(const char* ptr);
', __DIR__.'/../rust-core/target/release/libdelta1.so');

function delta1_infer(int $modelId, string $input): string {
  $ptr = $GLOBALS['ffi']->delta1_infer($modelId, $input);
  try { return FFI::string($ptr); }
  finally { $GLOBALS['ffi']->delta1_free_str($ptr); }
}
```

---

## Event- & logvelden

**Log (JSON)**

* `ts`, `req_id`, `actor`, `module`, `event`, `level`, `dur_ms`, `code`, `msg`

**Metrics**

* `infer_latency_ms`, `infer_qps`, `error_ratio`, `drift_psi`, `train_dur_ms`

---

## Foutcodes

| Code | Naam         | Betekenis kort          |
| ---: | ------------ | ----------------------- |
|    0 | Ok           | Succes                  |
|   10 | InvalidInput | Schema/validatie fout   |
|   20 | Io           | I/O of permissie        |
|   30 | NotFound     | Dataset/Model ontbreekt |
|   40 | Unauthorized | Geen rechten            |
|   50 | Internal     | Onverwachte fout        |

---

## Configuratie

**Bron**: ENV/TOML (immutable na start).

| Sleutel               | Voorbeeld                      |
| --------------------- | ------------------------------ |
| `DELTA1_DATA_ROOT`    | `/var/delta1`                  |
| `DELTA1_DB_DSN`       | `pgsql:host=...;dbname=delta1` |
| `DELTA1_DB_USER/PASS` | `delta1` / `***`               |
| `DELTA1_REGION`       | `eu-west`                      |
| `DELTA1_LOG_LEVEL`    | `info`                         |
| `DELTA1_TRAIN_SEED`   | `42`                           |
| `DELTA1_INFER_THRESH` | `0.65`                         |

---

## Testmatrix

| Laag         | Soort    | Wat                                      |
| ------------ | -------- | ---------------------------------------- |
| Rust/domain  | Unit     | Entiteiten, services, errors             |
| Rust/ffi     | Contract | Header ↔ symbolen, memory/ownership      |
| PHP          | Unit     | Validatie, PDO-queries (named params)    |
| Integratie   | E2E      | ingest→train→evaluate→infer→audit        |
| Conformiteit | Policies | DPIA-check, dataminimalisatie, retentie  |
| Fairness     | Eval     | disparate impact, thresholds per segment |

---

## Snelle referentie: module-grenzen

| Module       | Roept aan                     | Exporteert naar buiten |
| ------------ | ----------------------------- | ---------------------- |
| `data`       | `common`                      | via `api::ffi`         |
| `training`   | `data`, `common`              | via `api::ffi`         |
| `inference`  | `training`                    | via `api::ffi`         |
| `evaluation` | `data`,`training`,`inference` | (intern/rapport)       |
| `api::ffi`   | alle domeinen                 | C-ABI (PHP FFI)        |

---

## Beveiliging & privacy (per module, kernpunten)

* `data`: schema-strict, PII-hashing/pseudonymisatie, EU-residency.
* `training`: reproduceerbaarheid, modelkaart, artefact-encryptie.
* `inference`: rate-limit, threshold-gates, explain tokens.
* `evaluation`: bias-dash, drift-alerts, audit-export.
* `api::ffi`: minimale surface, FFI-safe types, semver en symbol whitelist.

---
