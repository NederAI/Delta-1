# Delta 1 — Architecture

> Hypermoderne, toekomstbestendige AI-architectuur met Europese normen & waarden als uitgangspunt. Kern: modulaire **Rust-monoliet**, **PHP-interface** via FFI, **zonder frameworks**. Dataminimalisatie, veiligheid-by-design, uitlegbaarheid en controleerbaarheid staan centraal.

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
+---------------------+        FFI (C-ABI)         +---------------------+
|     PHP Interface   |  <-----------------------> |      RUST Core      |
|  (HTTP endpoints,   |                            |  Modular Monolith   |
|   sessions, PDO)    |                            |  data/training/inf. |
+----------+----------+                            +----+---+---+---+---+
           |  HTTP (optioneel, zonder framework)        |   |   |   |
           v                                            |   |   |   |
        Clients                                   +-----+---+---+---+------+
 (webhooks/CLI/batch)                             |  Modules: data | train |
                                                  |  inference | eval | api|
                                                  +-------------+---------+
```

**Kernmodules (Rust)**

* `data`: acquisitie, validatie, normalisatie.
* `training`: modelbouw, hertrainen, versiebeheer.
* `inference`: realtime/batch voorspellingen, A/B.
* `evaluation`: metriek, bias checks, canaries.
* `api::ffi`: FFI-export voor PHP; **enige** cross-modulaire poort.
* `common`: config, errors, logging, telemetry.

**PHP-laag**

* Minimalistische endpoints (of alleen CLI) die FFI-functies aanroepen.
* PDO (named parameters) voor opslag/metadata.
* Geen frameworks; eenvoudige router/dispatcher.

---

## 3. Architectuurprincipes

* **Modulaire monoliet**: grenzen per domein, interne API’s; geen shared mutable state buiten modulegrenzen.
* **Hexagonaal**: domein logica ≠ infrastructuur; repositories als traits, implementaties in `infrastructure/`.
* **Contract-first** FFI: stabiele C-ABI, versie tags, semver.
* **Deterministisch & reproduceerbaar**: vaste seeds, vastgepinde toolchains.
* **Observability-first**: structured logs, metrics, traces (zonder externe frameworks: eigen, lichtgewicht appenders).

---

## 4. DDD snijvlakken

* **Contexten**: `Data`, `ModelMgmt`, `Inference`, `Evaluation`.
* **Entiteiten**: `Dataset`, `ModelVersion`, `Run`, `Prediction`.
* **Waardeobjecten**: `Schema`, `HyperParams`, `Thresholds`.
* **Use-cases**: `IngestDataset`, `TrainModel`, `Evaluate`, `Predict`.

---

## 5. Datastromen

1. **Acquisitie** (CSV/JSON/stream) → schemavalidatie → normalisatie → `Dataset` registreren.
2. **Training**: selecteer dataset+config → train → produceer `ModelVersion` (artefact + metadata).
3. **Evaluatie**: offline metrics (ROC/PR/AUC/F1), fairness-testen, canary-set.
4. **Publicatie**: model naar `ACTIVE` met versie-alias (bv. `current`).
5. **Inferentie**: request → validatie → model-aanroep → post-checks (confidence/thresholds) → antwoord + logging.
6. **Monitoring**: datadrift, performancedrift, incidentlog.

---

## 6. Opslag & schema

**Minimalistisch (EU-residency)**

* `postgres` (of sqlite dev) voor metadata:

  * `datasets(id, hash, schema_json, created_at)`
  * `models(id, version, created_at, metrics_json, bias_json, card_json, status)`
  * `runs(id, type, started_at, finished_at, meta_json, ok)`
  * `predictions(id, model_id, req_hash, result_hash, ts, meta_json)` *(eventueel geaggregeerd)*
* Artefacten op filesystem (`/var/delta1/models/<id>-<version>.bin`) of S3-compatibel **binnen EU**.

**Backup & retentie**

* PITR waar mogelijk; encrypt-at-rest; rotatie-beleid conform bewaartermijnen.

---

## 7. Beveiliging

* **In-transit** TLS (terminatie buiten scope of mTLS tussen componenten).
* **At-rest**: disks/artefacten versleuteld; secrets als env-vars + sealed files.
* **Least-privilege** gebruikers/rollen; geen world-writable paths.
* **FFI-hardened**: alleen FFI-veilige types; bounds-checks; geen ongetype pointeraritmetiek.
* **Input-sanitatie** en schema-validatie in `data` module.
* **Audittrail**: onmutable append-log voor gevoelige operaties (train/publish).

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
* **Consent/contract**: consent-status deel van datamodel; geen gebruik buiten scope.

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
* Alleen POD-structs; strings als `const char*` (UTF-8), eigenaarschap gedocumenteerd.

**Voorbeeld**

```c
// Header (voor PHP FFI)
unsigned int delta1_api_version(void);

unsigned int delta1_data_ingest(const char* path, const char* schema_json);
/* return dataset_id (>0) of 0 bij fout */

unsigned int delta1_train(unsigned int dataset_id, const char* config_json);
/* return model_id */

const char*  delta1_infer(unsigned int model_id, const char* input_json);
/* return JSON; call delta1_free_str(ptr) when done */

void         delta1_free_str(const char* ptr);
```

---

## 13. PHP-laag (zonder frameworks)

* **Endpoints/CLI** roepen FFI-functies aan; validatie + auth in PHP.
* **PDO** met named parameters voor meta-opslag en audit.
* **Rate-limiting** en simpele tokenauth (liefst mTLS voor interne calls).
* **Geen globale staat**; alles via request-scope.

---

## 14. Teststrategie

* **Unit**: per module (Rust `cargo test`).
* **FFI-contracttests**: tegen de cdylib met golden headers.
* **Integratie**: PHP → FFI → Rust → opslag.
* **E2E**: ingest → train → publish → infer → monitor.
* **Conformiteit**: DPIA-checklist, dataminimalisatie-linting (schema-diffs), bias-testen.

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
* [ ] Modelkaart ingevuld
* [ ] Rollback-punt beschikbaar

**Operations**

* [ ] Drift-alerts actief
* [ ] Backups recent getest
* [ ] Audittrail intact
* [ ] Retentie en purges lopen

---

## 20. Minimale voorbeeldsignatures (Rust)

```rust
#[no_mangle]
pub extern "C" fn delta1_api_version() -> u32 { 1 }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(path: *const c_char, schema: *const c_char) -> u32 { /* ... */ }

#[no_mangle]
pub extern "C" fn delta1_train(dataset_id: u32, cfg: *const c_char) -> u32 { /* ... */ }

#[no_mangle]
pub extern "C" fn delta1_infer(model_id: u32, input: *const c_char) -> *const c_char { /* ... */ }

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char) { /* ... */ }
```

---

## 21. Migratie naar services (optioneel, later)

* Kies module met hoogste schaalbehoefte (bv. `inference`).
* Exporteer bestaand intern contract als netwerk-API (zelfde payloads).
* Behoud modelkaart/monitoring/audit ongewijzigd.
