# Delta 1 — Rust Core Architectuur

> Modulaire monoliet in *pure std Rust* (geen frameworks of externe crates). Stabiliteit via een smalle, C-ABI FFI-laag. Sterke grenzen per domein, zero-copy waar mogelijk, en eenvoudige, testbare componenten.

---

## 1. Scope & Doelstellingen

* **Scope**: `rust-core/` (library `cdylib`) met domeinen `data`, `training`, `inference`, `evaluation`, ondersteund door `common` en een dunne `api::ffi` laag.
* **Doelen**: deterministisch, memory-safe, minimale dependencies (alleen std), stabiele ABI, hoge performance, eenvoudige observability, EU-conforme privacy/veiligheid.

---

## 2. Moduleboom

```
rust-core/
└── src/
    ├── lib.rs                 # orchestrator, re-exports
    ├── common/
    │   ├── mod.rs
    │   ├── error.rs           # DeltaError, DeltaCode
    │   ├── config.rs          # ENV/kv-config
    │   ├── time.rs            # klokhelpers, monotonic timing
    │   ├── log.rs             # lichte JSON logging (stdout/stderr)
    │   ├── buf.rs             # eenvoudige buffer/arena hulpen
    │   └── ids.rs             # id/hash helpers
    ├── data/
    │   ├── mod.rs
    │   ├── domain.rs          # Dataset, Schema
    │   ├── service.rs         # ingest, validate, normalize
    │   └── repo_fs.rs         # fs-gebaseerde opslag (std::fs)
    ├── training/
    │   ├── mod.rs
    │   ├── domain.rs          # ModelVersion, TrainConfig
    │   ├── service.rs         # train(), versioneer, materialiseer
    │   └── repo_fs.rs         # artefact-IO (bin blobs)
    ├── inference/
    │   ├── mod.rs
    │   ├── domain.rs          # routes, thresholds
    │   ├── service.rs         # infer(), batch_infer()
    │   └── workers.rs         # lichtgewicht worker-pool (std::thread/mpsc)
    ├── evaluation/
    │   ├── mod.rs
    │   ├── domain.rs          # EvalSuite, DriftStats
    │   └── service.rs         # evaluate(), drift()
    └── api/
        ├── mod.rs
        └── ffi.rs             # #[no_mangle] extern "C"
```

---

## 3. Cross-module contracten (interne traits)

> Geen runtime DI; compile-time grenzen via traits en `pub(crate)`.

```rust
// data/domain.rs
pub struct DatasetId(pub u32);
pub struct Dataset { pub id: DatasetId, pub schema_json: String, pub created_ms: u128 }

pub trait DataRepo {
    fn put_dataset(&self, d: &Dataset) -> Result<(), DeltaError>;
    fn get_dataset(&self, id: DatasetId) -> Result<Dataset, DeltaError>;
}

// training/domain.rs
pub struct ModelId(pub u32);
pub struct ModelVersion { pub id: ModelId, pub version: String, pub artefact_path: String }

pub trait ModelRepo {
    fn put_model(&self, m: &ModelVersion) -> Result<(), DeltaError>;
    fn get_model(&self, id: ModelId) -> Result<ModelVersion, DeltaError>;
}

// inference/domain.rs
pub struct Prediction { pub json: String, pub latency_ms: u32, pub confidence: f32 }
pub trait InferEngine {
    fn infer(&self, model: &ModelVersion, input_json: &str) -> Result<Prediction, DeltaError>;
}
```

Implementaties zitten in `repo_fs.rs` en `service.rs`. `lib.rs` composeert concrete structen en stelt alleen *smalle* façade-functies publiek.

---

## 4. Foutafhandeling

* Eén type: `DeltaError` + numerieke `DeltaCode` (stabiele mapping voor FFI).
* Geen panics over modulegrenzen; alles via `Result<_, DeltaError>`.

```rust
// common/error.rs
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum DeltaCode { Ok=0, InvalidInput=10, Io=20, NotFound=30, Internal=50 }

pub struct DeltaError { pub code: DeltaCode, pub msg: &'static str }

impl DeltaError {
    pub fn io() -> Self { Self{ code:DeltaCode::Io, msg:"io" } }
    pub fn invalid(m:&'static str)->Self{ Self{code:DeltaCode::InvalidInput,msg:m} }
}
```

---

## 5. Configuratie (zonder externe parser)

* Bron: **ENV** variabelen en optioneel `key=value` bestand (eenvoudig, line-based).
* Immutable runtime snapshot.

```rust
// common/config.rs
pub struct AppCfg {
    pub data_root: String,      // bv. /var/delta1
    pub region:    String,      // bv. eu-west
    pub log_level: u8,          // 0=error..3=debug
}
pub fn load_cfg() -> AppCfg {
    fn env(k:&str, d:&str)->String { std::env::var(k).unwrap_or_else(|_| d.into()) }
    AppCfg {
        data_root: env("DELTA1_DATA_ROOT", "./data"),
        region:    env("DELTA1_REGION",    "eu"),
        log_level: env("DELTA1_LOG_LEVEL", "1").parse().unwrap_or(1),
    }
}
```

---

## 6. Logging & Observability (lichtgewicht)

* JSON-regels naar stdout; geen lock-contentie door korte, geformatteerde strings.
* Korrel: module/event, duur, code.

```rust
// common/log.rs
pub fn log_json(level:&str, module:&str, event:&str, code:u32, dur_ms:u128) {
    let ts = crate::common::time::now_ms();
    println!("{{\"ts\":{ts},\"level\":\"{level}\",\"mod\":\"{module}\",\"ev\":\"{event}\",\"code\":{code},\"dur_ms\":{dur_ms}}}");
}
```

---

## 7. Concurrency-model

* CPU-bound taken: *worker-pool* in `inference/workers.rs` met `std::sync::mpsc`.
* IO-bound: synchrone std IO, backpressure via queue-lengte.
* Geen globale mutable state; gedeelde structen via `Arc<...>` + `Mutex/RwLock` alleen waar strikt nodig.

```rust
// inference/workers.rs
pub struct Pool {
    tx: std::sync::mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>
}
impl Pool {
    pub fn new(n:usize)->Self{
        let (tx, rx)=std::sync::mpsc::channel::<Box<dyn FnOnce()+Send>>();
        for _ in 0..n {
            let rx=rx.clone();
            std::thread::spawn(move || while let Ok(job)=rx.recv(){ job(); });
        }
        Self{tx}
    }
    pub fn submit<F:FnOnce()+Send+'static>(&self, f:F){ let _=self.tx.send(Box::new(f)); }
}
```

---

## 8. Data-pad (minimalistisch)

* **Ingest**: lees bestand in streaming modus (`BufRead::lines()`), simpele CSV/JSON-heuristiek, schema-check (veldnamen/typen zoals gedeclareerd in `schema_json`).
* **Normalize**: trimming, lowercasing waar passend, PII-strategieën (hash/pseudonymize) voordat iets wordt opgeslagen op disk.
* **Opslag**: metadata in eenvoudige `*.meta` bestanden (line-based of mini-JSON) + datahash als sleutelnaam.

```rust
// data/service.rs (schets)
pub fn ingest_file(path:&str, schema_json:&str) -> Result<DatasetId, DeltaError> {
    use std::{fs::File, io::{BufRead,BufReader}};
    let f = File::open(path).map_err(|_| DeltaError::io())?;
    let mut rdr = BufReader::new(f);
    let mut hasher = crate::common::ids::SimpleHash::new();
    let mut count = 0u64;
    let mut line = String::new();
    while rdr.read_line(&mut line).map_err(|_|DeltaError::io())? > 0 {
        hasher.update(line.as_bytes()); // normalisatie kan hier
        count += 1; line.clear();
    }
    let id = DatasetId(hasher.finish32());
    // schrijf meta naar {data_root}/datasets/{id}.meta
    // ...
    Ok(id)
}
```

---

## 9. Train-pad (std-only)

> *Opzettelijk eenvoudig*: we modelleren training als transformatie van dataset → artefact (binaire blob). De “ML” kan later achter FFI worden gehaakt; hier gaat het om lifecycle & versiebeheer.

* `TrainConfig`: hyperparameters als mini-JSON string (gemaakt in PHP; parsing in Rust enkel wat we nodig hebben via eenvoudige substring/number-extractie om externe JSON te vermijden).
* Artefact: binaire file `{models}/{id}-{version}.bin` met header (magic, versie, checksum).

```rust
// training/service.rs (schets)
pub fn train(dataset: DatasetId, cfg_json:&str) -> Result<ModelId, DeltaError> {
    // 1) laad dataset meta/gegevens
    // 2) simuleer “training”: produceer deterministische bytes o.b.v. seed
    // 3) schrijf artefact + modelkaart (.card)
    Ok(ModelId( /* hash */ 1 ))
}
```

---

## 10. Infer-pad

* Laad `ModelVersion` → `InferEngine::infer()` → retourneer `Prediction` met JSON-string (manueel geformatteerd om geen externe JSON-lib te gebruiken).
* Batch-variant accepteert array-input in één call voor throughput.

```rust
// inference/service.rs (schets)
pub fn infer(model: &ModelVersion, input_json:&str) -> Result<Prediction, DeltaError> {
    let start=crate::common::time::now_ms();
    // dummy: echo input met stub confidence
    let out = format!("{{\"ok\":true,\"model\":\"{}\",\"y\":null}}", model.version);
    let dur = (crate::common::time::now_ms() - start) as u32;
    Ok(Prediction{ json: out, latency_ms: dur, confidence: 0.5 })
}
```

---

## 11. Evaluatie & Drift

* Evaluatie produceert compacte rapporten (`metrics.card`, `bias.card`) met kerncijfers (AUC/F1 worden hier als placeholders berekend met eenvoudige tellers).
* Drift: simpele PSI/KS-benadering op basis van histogrammen die tijdens inferentie worden geaccumuleerd (in geheugen, periodiek flush naar disk).

---

## 12. FFI-grens (ABI-stabiel)

* Alleen **POD** types over de grens: `u32`, `*const c_char`.
* Strings richting PHP: gealloceerd via `CString::into_raw()`, vrijgave via `delta1_free_str`.

```rust
// api/ffi.rs
use std::os::raw::c_char;
use std::ffi::{CStr, CString};

#[no_mangle] pub extern "C" fn delta1_api_version()->u32 { 1 }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(path:*const c_char, schema:*const c_char)->u32{
    let p = unsafe{ CStr::from_ptr(path) }.to_string_lossy();
    let s = unsafe{ CStr::from_ptr(schema) }.to_string_lossy();
    match crate::data::service::ingest_file(&p, &s){ Ok(id)=>id.0, Err(_)=>0 }
}

#[no_mangle]
pub extern "C" fn delta1_infer(model_id:u32, input:*const c_char)->*const c_char{
    let inp = unsafe{ CStr::from_ptr(input) }.to_string_lossy();
    let mv = crate::training::service::load_model(ModelId(model_id)).unwrap();
    let out = crate::inference::service::infer(&mv, &inp)
        .map(|p| p.json).unwrap_or_else(|_| "{\"ok\":false}".into());
    CString::new(out).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr:*const c_char){
    if ptr.is_null(){return;}
    unsafe{ let _ = CString::from_raw(ptr as *mut c_char); }
}
```

---

## 13. Geheugen & Zero-copy Richtlijnen

* Geef voorkeur aan `&[u8]`/`&str` boven `Vec` waar mogelijk.
* Hergebruik buffers (zie `common::buf`) voor I/O-loops.
* Vermijd onnodige klonen; gebruik `Cow` niet (std-only), dus handmatig met slices werken.
* In FFI: *alleen* één keer alloceren aan de Rust-kant; call-site in PHP *moet* vrijgeven.

---

## 14. Beveiliging (kern)

* Pad-sanitatie bij file-I/O: verbied `..` path traversal; prefix alle paden met `cfg.data_root`.
* Bestandsrechten minimaal; fail-fast bij permissieproblemen.
* Input-validatie: schema-namen/kolommen alleen `[A-Za-z0-9_]+`.
* Crash-isolation: geen unwrap buiten test/scope; `Result`/`Option` strikt checken.

---

## 15. Performance-playbook

* Ingest: buffer size afstemmen (64–256 KiB), lijn-gebaseerde parsing.
* Infer: batch API; worker-pool met N = cores.
* I/O: append-only voor logs/artefacten; lock duur minimaliseren.
* Build: `-C lto=fat` (release), `panic=abort` in `Cargo.toml` voor cdylib.

```toml
# Cargo.toml (relevant excerpt)
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

---

## 16. Teststrategie

* **Unit**: elke `domain.rs`/`service.rs` met pure std tests.
* **FFI-contract**: symbol-aanwezigheid + round-trip string alloc/free.
* **Integratie**: ingest→train→infer met temp-directories (`std::env::temp_dir()`).
* **Leak-check**: voor elke FFI string: alloc → `delta1_free_str`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn err_codes_are_stable(){ assert_eq!(DeltaCode::Ok as u32, 0); }
}
```

---

## 17. Build & Lay-out

* **cdylib** output:

  * Linux: `libdelta1.so`
  * macOS: `libdelta1.dylib`
  * Windows: `delta1.dll`
* *Geen* `serde`, *geen* `regex`, *geen* async runtime; alles std.

---

## 18. Publieke façade (binnen crate)

Alleen deze drie façade-functies worden via `lib.rs` her-geëxporteerd (intern bruikbaar én door FFI aangeroepen):

```rust
// lib.rs
pub use data::service::ingest_file as core_data_ingest;
pub use training::service::{train, load_model};
pub use inference::service::infer as core_infer;
```

---

## 19. Minimale skeletimplementaties (knip-en-plak start)

```rust
// common/time.rs
pub fn now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

// common/ids.rs
pub struct SimpleHash(u32);
impl SimpleHash {
    pub fn new()->Self{ Self(2166136261) } // FNV basis
    pub fn update(&mut self, bytes:&[u8]){ for b in bytes { self.0 = (self.0 ^ (*b as u32)).wrapping_mul(16777619); } }
    pub fn finish32(&self)->u32{ self.0 }
}
```

---

## 20. Migratiepad (wanneer nodig)

* Module extraheren → eigen proces → netwerkcontract blijft identiek aan FFI-payloads (JSON strings).
* De interne traits (`DataRepo`, `ModelRepo`, `InferEngine`) worden dan “drivers” naar IPC/HTTP zonder de domeinlogica te wijzigen.

---
