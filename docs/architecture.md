# Delta 1 — Architecture

> Hypermoderne, toekomstbestendige AI-architectuur met Europese normen & waarden als uitgangspunt. Kern: modulaire **Rust-monoliet**, **PHP-interface** via FFI, **zonder frameworks**. De nieuwste iteratie introduceert consent-gedreven inferentiecontext, deterministische routing, fairness/DP-gates en exporteerbare modeldocumentatie. Dataminimalisatie, veiligheid-by-design, uitlegbaarheid en controleerbaarheid staan centraal.

---

## 1. Doelen & niet-doelen

**Doelen**

* Lage operationele complexiteit, hoge performance en memory-safety (Rust).
* Heldere grenzen tussen domeinen (DDD + hexagonale principes).
* Privacy-by-design: dataminimalisatie, purpose-limitation, consent/contractuele grondslag.
* Eenvoudig pad naar opsplitsen in services wanneer nodig (module-extractie).
* Transparante AI: modelkaarten, evals, monitoring, audittrail.

**Niet-doelen**

* Geen “big-bang microservices”.
* Geen zware frameworks of runtime-magie.
* Geen vendor lock-in; alle artefacten lokaal te bouwen.

---

## 2. Systeemoeverzicht

```
+---------------------------+   C-ABI (DeltaCode + JSON/char**)   +----------------------------+
|       PHP Interface       | <-------------------------------->  |          RUST Core         |
| - HTTP/CLI zonder fw      |                                    | - Modulaire monoliet        |
| - PDO consent/audit/meta  |                                    | - Data/Training/Inferentie  |
| - Audit ledger & tooling  |                                    | - Router, DP, WhyLog, docs  |
+------------+--------------+                                    +-----+---+---+---+---+-----+
             |  HTTP/CLI/batch zonder framework                           |   |   |   |   |
             v                                                            |   |   |   |   |
        Clients & integraties                                    +-------+---+---+---+---+-------+
                                                                  | data | training | inference |
                                                                  | evaluation | api::ffi | common |
                                                                  +--------------------------------+
```

**Kernmodules (Rust)**

* `data`: bestandsingestie, hashing, datasheet-export (`export_datasheet`).
* `training`: fairness- en DP-gates, deterministische model-id, in-memory registry + modelcard-export.
* `inference`: consent-checks, SSM-router (tabular/text), fallback, WhyLog-hash en saliency.
* `evaluation`: metriek/bias placeholders richting rapportages.
* `api::ffi`: FFI-export met stabiele `DeltaCode`-statussen, `char**`-uitvoer en `delta1_free_str`.
* `common`: config, errors, hashing, tijd, mini JSON-utils voor zero-deps parsing.

**PHP-laag**

* Minimalistische HTTP/CLI-entrypoints die FFI-functies (`delta1_*`) aanroepen en `DeltaCode` mappen naar HTTP-status.
* PDO (named parameters) voor consent, audit ledger en metadata.
* Header/preload-strategie voor veilige `FFI::load`, plus char-pointerbeheer (`delta1_free_str`).

---

## 3. Architectuurprincipes

* **Modulaire monoliet**: grenzen per domein, interne API’s; geen shared mutable state buiten modulegrenzen.
* **Hexagonaal**: domein logica ≠ infrastructuur; repositories als traits, implementaties in `infrastructure/`.
* **Contract-first** FFI: stabiele C-ABI, `DeltaCode`-statussen, `char**`-uitvoer, semver (`delta1_api_version`).
* **Consent & governance by design**: inferentie vereist `purpose_id` + `subject_id`, audit-ready WhyLog-hash.
* **Deterministisch & reproduceerbaar**: vaste seeds, vastgepinde toolchains, deterministische id-hashing.
* **Observability-first**: structured logs, metrics, traces (zonder externe frameworks: eigen, lichtgewicht appenders) + document-export (modelcard/datasheet).

---

## 4. DDD snijvlakken

* **Contexten**: `Data`, `ModelMgmt`, `Inference`, `Evaluation`.
* **Entiteiten**: `Dataset`, `ModelVersion`, `Run`, `Prediction`.
* **Waardeobjecten**: `Schema`, `HyperParams`, `Thresholds`.
* **Use-cases**: `IngestDataset`, `TrainModel`, `Evaluate`, `Predict`.

---

## 5. Datastromen

1. **Acquisitie & datasheet**: CSV/JSON/stream → schemavalidatie → normalisatie → hashing → `DatasetId` (string) via `delta1_data_ingest` → `export_datasheet` voor audittrail.
2. **Training & policies**: kies dataset + config → `training::service::train` → DP/fairness-gates → `ModelVersion` (artefactpad + metadata) in in-memory registry.
3. **Activatie**: `delta1_load_model` laadt (eventueel specifieke versie) en registreert actief model voor inferentie.
4. **Inferentie met context**: `delta1_infer_with_ctx(purpose, subject, payload)` → consent-check → SSM-router (tabular/text) → fallback indien nodig → JSON-respons met route, confidence, WhyLog-hash.
5. **Documentatie**: `delta1_export_model_card` en `delta1_export_datasheet` leveren governancedocumenten (JSON) richting PHP/ops.
6. **Monitoring & audit**: latencies, drift, incidentlog, auditledger met WhyLog-hash en consentbeslissingen.

---

## 6. Opslag & schema

**Minimalistisch (EU-residency)**

* `postgres` (of sqlite dev) voor metadata:

  * `datasets(id, hash, schema_json, created_at)`
  * `models(id, version, created_at, metrics_json, bias_json, card_json, status)`
  * `consent(subject_hash, purpose_id, status, expires_at)`
  * `audit_log(id, event_time, model_id, version, purpose_id, subject_ref, merkle_root, whylog_hash)`
  * `runs(id, type, started_at, finished_at, meta_json, ok)` *(optioneel)*
  * `predictions(id, model_id, req_hash, result_hash, ts, meta_json)` *(eventueel geaggregeerd)*
* Artefacten op filesystem (`/var/delta1/models/<id>/<version>/model.bin`) + tokenizers; datasheets/modelcards als JSON.
* Huidige implementatie houdt modelregistry/datasheets in-memory; persistente repos worden later aangesloten op dezelfde contracts.

**Backup & retentie**

* PITR waar mogelijk; encrypt-at-rest; rotatie-beleid conform bewaartermijnen.

---

## 7. Beveiliging

* **In-transit** TLS (terminatie buiten scope of mTLS tussen componenten).
* **At-rest**: disks/artefacten versleuteld; secrets als env-vars + sealed files.
* **Least-privilege** gebruikers/rollen; geen world-writable paths.
* **FFI-hardened**: alleen FFI-veilige types; bounds-checks; `char**` output met eigenaarschap + verplicht `delta1_free_str`.
* **Input-sanitatie** en schema-validatie in `data` module.
* **Audittrail**: onmutable append-log voor gevoelige operaties (train/publish/infer) met WhyLog-hash.

---

## 8. Privacy, ethiek & EU-conforme governance

* **GDPR-alignment**:

  * *Lawful basis* documenteren per gegevensstroom.
  * *Data minimization* en *purpose limitation* afdwingen in `data`.
  * *DPIA* template voor nieuwe use-cases.
  * *Data subject rights* (inzage, verwijdering): indexeren zodat verzoeken efficiënt zijn.
  * *Data residency*: fysieke opslag in EU; geen overdracht buiten EU zonder passende waarborgen.
* **EU AI-principes** (in lijn met EU AI-wetgeving en ENISA-aanbevelingen):

  * Risicoclassificatie per systeem (laag/hoog/middel).
  * *Human-in-the-loop* waar passend (drempel/flag vereist handmatige review).
  * *Explainability*: modelkaart, feature-belang, decision rationale op record-niveau (post-hoc).
  * *Bias & non-discrimination*: representativiteit, disparate impact, equalized odds waar relevant.
  * *Traceability*: alle trainingsruns, datasets, hyperparams en commits gelogd.
* **Retentie & doeleinden**: bewaartermijnen per dataset; automatische purges.
* **Consent/contract**: consent-status deel van datamodel (`purpose_id`, gehashte `subject_id`, status=`granted|denied|expired`); geen gebruik buiten scope.

---

## 9. Modellevenscyclus (MLOps-light, zonder zware tooling)

* **Versiebeheer**: elke build → `ModelVersion` met hash van code+data+config.
* **Evaluaties**: vaste suite + fairness-checks; gate op min. drempels (policy).
* **Canary release**: percentage route (in `inference`) met shadow logging.
* **Rollbacks**: alias terug naar vorige versie; artefacten blijven bewaard.
* **Drift-monitoring**: PSI/KS/JS-divergence per feature; alarmen bij overschrijding.
* **Red-teaming**: periodieke adversarial tests; prompt/poisoning scenario’s (indien LLM).

---

## 10. Observability

* **Structured logging** (JSON): `ts, req_id, actor, module, event, dur_ms`.
* **Metrics**: latency, QPS, error ratio, cache hitrate, drift-scores.
* **Traces**: eenvoudige span-ids door FFI heen (propagatie in headers/ctx).
* **Audit events**: WhyLog-hash + consentbeslissing naar append-only ledger.
* **SLO’s**: p95 latency (inferentie), beschikbaarheid, foutbudget.

---

## 11. Schaalbaarheid & performance

* **Binnen monoliet**: worker-pools, zero-copy waar mogelijk, batch-inferentie pad.
* **IO-patterns**: streaming parsers, backpressure.
* **CPU/GPU**: optionele FFI naar gespecialiseerde libs of subprocess (zonder runtime frameworks).
* **Extractie-pad**: module → eigen proces → netwerkcontract blijft gelijk.

---

## 12. FFI-contract (C-ABI, stable)

**Naming & versies**

* Symboolprefix `delta1_`; semver exporttabel (`delta1_api_version()`).
* Alleen POD-structs; strings als `const char*` (UTF-8), eigenaarschap gedocumenteerd (`delta1_free_str`).
* Statuscodes volgens `DeltaCode`: `0=Ok`, `1=NoConsent`, `2=PolicyDenied`, `3=ModelMissing`, `4=InvalidInput`, `5=Internal`.

**Voorbeeld**

```c
// Header (voor PHP FFI)
const char* delta1_api_version(void); // "1.0.0"

int delta1_data_ingest(const char* filepath, char** out_dataset_id);
// DeltaCode; bij succes wordt *out_dataset_id toegewezen (caller vrijgeven)

int delta1_train(const char* dataset_id, const char* train_cfg_json, char** out_model_id);
// DeltaCode; model-id als string terug via out-parameter

int delta1_load_model(const char* model_id, const char* version_opt);
// DeltaCode; houdt actief model in geheugen voor inferentie

const char* delta1_infer_with_ctx(
    const char* purpose_id,
    const char* subject_id,
    const char* input_json
);
// JSON-response (WhyLog, route, confidence); caller moet free doen

const char* delta1_export_model_card(const char* model_id);
const char* delta1_export_datasheet(const char* dataset_id);

void delta1_free_str(const char* ptr);
```

---

## 13. PHP-laag (zonder frameworks)

* **Endpoints/CLI** roepen `delta1_data_ingest/train/load_model/infer_with_ctx/export_*` aan; validatie + auth in PHP.
* **DeltaCode → HTTP**: mapping (`0=200`, `1=403`, `2=422/403`, `3=404`, `4=400`, `5=500`).
* **PDO** met named parameters voor consent, audit-log, metadata.
* **FFI-beheer**: preload header, `FFI::load`, `FFI::string` + `delta1_free_str` wrappers voor alle `const char*`-returns.
* **Rate-limiting** en simpele tokenauth (liefst mTLS voor interne calls).
* **Geen globale staat**; alles via request-scope (actieve model-id in Rust via `delta1_load_model`).

---

## 14. Teststrategie

* **Unit**: per module (Rust `cargo test`).
* **FFI-contracttests**: cdylib ↔ header; `char**`-outparams, `DeltaCode`-mapping, `infer_with_ctx` JSON.
* **Integratie**: PHP → FFI → Rust → consent/audit/artefact.
* **E2E**: ingest → train → load_model → infer_with_ctx → export docs → monitor.
* **Conformiteit**: DPIA-checklist, dataminimalisatie-linting (schema-diffs), bias-testen, consent-deny paths.

---

## 15. Deploy & omgevingen

* **Omgevingen**: `dev`, `staging` (EU), `prod` (EU).
* **Build**: reproduceerbare Rust toolchain; release cdylib + checksums.
* **Runtime**: één container of bare-metal; geen extra orchestrators verplicht.
* **Secrets**: via env/keystore; nooit in repo.
* **Blue/Green**: symlink-switch van actieve cdylib + alias-update in metadata.

---

## 16. Configuratie

* TOML/ENV met strikte parsing; immutable na start.
* Feature-flags per module (enable/disable).
* Thresholds en policies in versiebeheer (auditbaar).

---

## 17. Beheer & operaties

* **Runbooks**: incidenten, rollbacks, schema-migraties.
* **Backups**: dagelijkse full + incrementeel; restore-oefeningen.
* **Retentie**: logs/artefacten conform beleid; automatische purges.
* **Toegangsbeheer**: minimaal rechtensysteem, 2-man rule voor publish.

---

## 18. Accessibility, i18n, transparantie

* **Uitlegschermen**: brondata, beperkingen, onzekerheid (confidence).
* **Taal**: i18n strings; EU-talen waar nodig.
* **Toegankelijkheid**: WCAG-principes voor frontends (indien aanwezig).

---

## 19. Checklists

**Pre-train**

* [ ] Doel en rechtsgrond gedefinieerd
* [ ] DPIA uitgevoerd (indien vereist)
* [ ] Dataset gedocumenteerd & geminimaliseerd
* [ ] Bias-risico’s geïnventariseerd

**Pre-publish**

* [ ] Evals ≥ drempels
* [ ] Fairness OK / mitigaties vastgelegd
* [ ] Modelkaart + datasheet geëxporteerd (`delta1_export_*`)
* [ ] Rollback-punt beschikbaar

**Operations**

* [ ] Drift-alerts actief
* [ ] Backups recent getest
* [ ] Audittrail intact (WhyLog-hash + consentstatus)
* [ ] Retentie en purges lopen

---

## 20. Minimale voorbeeldsignatures (Rust)

```rust
#[no_mangle]
pub extern "C" fn delta1_api_version() -> *const c_char { VERSION.as_ptr() }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(path: *const c_char, out_id: *mut *const c_char) -> i32 {
    if path.is_null() || out_id.is_null() {
        return DeltaCode::InvalidInput as i32;
    }
    match core_data_ingest(cstr(path), "{}") {
        Ok(id) => assign_out_string(out_id, id.into_inner()),
        Err(err) => err.code as i32,
    }
}

#[no_mangle]
pub extern "C" fn delta1_train(
    dataset_id: *const c_char,
    cfg_json: *const c_char,
    out_model_id: *mut *const c_char,
) -> i32 { /* DeltaCode + out-param */ }

#[no_mangle]
pub extern "C" fn delta1_infer_with_ctx(
    purpose_id: *const c_char,
    subject_id: *const c_char,
    input_json: *const c_char,
) -> *const c_char { /* JSON (WhyLog, route, confidence) */ }

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char) { /* consume CString::from_raw */ }
```

---

## 21. Migratie naar services (optioneel, later)

* Kies module met hoogste schaalbehoefte (bv. `inference`).
* Exporteer bestaand intern contract als netwerk-API (zelfde payloads).
* Behoud modelkaart/monitoring/audit ongewijzigd.
