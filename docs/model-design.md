Top-down, kort en scherp. Dit zijn de **definitieve keuzes** voor Delta-1.

# 1) Modellen & router

* **Tabular (default):** `smartcore`

  * LogisticRegression (L2) voor binaire taken.
  * GradientBoosting (stumps) voor niet-lineair.
  * Determinisme: `seed=42`, `f32`, vaste feature-volgorde.
* **Tekst (optioneel):** `candle` met **MiniLM-L6-v2 Q4** (alleen inferentie).

  * Ingebouwde eenvoudige tokenizer (whitespace + subword map op disk).
* **Router-regels (`router::SSMRouter`):**

  * Als input JSON een veld `"text"` bevat en `len(text) > 256` ⇒ **MiniLM**; anders tabular.
  * Als `features_only=true` in context ⇒ **tabular** (hard override).
  * Fallback bij model-miss ⇒ **logistic**.

# 2) Privacy & consent

* **Dataminimalisatie:** standaard verwijder ruwe request-payload na **24u** (gehashte features ok).
* **Consent:** vereis `purpose_id` + `subject_id` (hashbaar) → lookup in `consent` tabel (status=granted|denied|expired). Geen consent ⇒ 403 met `DeltaCode::NoConsent`.
* **Redactie vóór opslag/logging:**

  * E-mail, telefoon, IBAN, adres (regex-set), Unicode-normalisatie (NFKC).
  * Redactie-flag in WhyLog.
* **Differential Privacy (training):** DP-SGD: `epsilon=3.0`, `delta=1e-5`, `clip=1.0`, `noise=gaussian`. Uit/aan via `TrainConfig.dp=true|false`.

# 3) Fairness & gating

* **Meetwaarden per subgroep (`group` attribuut, optioneel):** ΔTPR ≤ **0.05**, ΔFPR ≤ **0.03**, ΔPPV ≤ **0.04**.
* **Mitigatie (volgorde):** reweighing → post-processing equalized odds.
* **CI-gate:** build faalt bij overschrijding of ontbrekende subgroup-rapportage.

# 4) Uitlegbaarheid (WhyLog)

* **Tabular:** top-5 feature-bijdragen via surrogaat (lineair fit op lokale buurt).
* **Tekst:** top-k tokenscores (saliency) + zinslange rationale (surrogate).
* **WhyLog-hash:** BLAKE3 over canonical JSON; teruggegeven in API en geaudit.

# 5) Audit & provenance

* **Append-only JSONL ledger** onder `${DATA_ROOT}/audit/`.
* **Merkle-ketting:** per 1.000 events een root; signeren met **Ed25519** (key in tmpfs).
* **Schema (minimaal):**

  ```json
  {"ts": "...", "event":"infer", "model_id":"...", "version":"...", "purpose":"...", "subject_hash":"...", "lat_ms":12, "whylog_hash":"...", "merkle_root":"..."}
  ```

# 6) Artefact- en datapaden

* Datasets: `${DATA_ROOT}/datasets/{dataset_id}/meta.json`
* Modellen: `${DATA_ROOT}/models/{model_id}/{version}/model.bin`
* Tokenizer: `${DATA_ROOT}/models/{model_id}/{version}/tokenizer.json`
* Audit: `${DATA_ROOT}/audit/YYYY-MM/ledger.jsonl`

# 7) FFI & API-contract

* **Exports (C-ABI):**

  ```c
  // versies
  const char* delta1_api_version(); // semver "1.0.0"

  // data
  int         delta1_data_ingest(const char* filepath, char** out_dataset_id);

  // training
  int         delta1_train(const char* dataset_id, const char* train_cfg_json, char** out_model_id);
  int         delta1_load_model(const char* model_id, const char* version); // warm

  // inferentie (met context)
  const char* delta1_infer_with_ctx(const char* purpose_id, const char* subject_id, const char* input_json);

  // documenten
  const char* delta1_export_model_card(const char* model_id);
  const char* delta1_export_datasheet(const char* dataset_id);

  // helper
  void        delta1_free_str(const char* s);
  ```
* **Retourcodes (stabiel):** `0=OK, 1=NoConsent, 2=PolicyDenied, 3=ModelMissing, 4=InvalidInput, 5=Internal`.

# 8) PHP-laag (zonder frameworks, PDO named params)

**DDL (PostgreSQL):**

```sql
CREATE TABLE consent(
  subject_hash CHAR(64) PRIMARY KEY,
  purpose_id   TEXT NOT NULL,
  status       TEXT NOT NULL CHECK (status IN ('granted','denied','expired')),
  expires_at   TIMESTAMPTZ
);

CREATE TABLE audit_log(
  id BIGSERIAL PRIMARY KEY,
  event_time TIMESTAMPTZ NOT NULL,
  model_id   TEXT NOT NULL,
  version    TEXT NOT NULL,
  purpose_id TEXT NOT NULL,
  subject_ref CHAR(64) NOT NULL,
  merkle_root CHAR(64) NOT NULL,
  whylog_hash CHAR(64) NOT NULL
);
```

**Queries (named params):**

```php
$ins = $pdo->prepare(
 "INSERT INTO audit_log(event_time,model_id,version,purpose_id,subject_ref,merkle_root,whylog_hash)
  VALUES (:t,:m,:v,:p,:s,:r,:w)"
);
$ins->execute([
  ':t'=>$ts, ':m'=>$mid, ':v'=>$ver, ':p'=>$purpose, ':s'=>$subjHash, ':r'=>$root, ':w'=>$whyHash
]);
```

# 9) Config & resources

* **ENV:** `DELTA1_DATA_ROOT=/var/lib/delta1`, `DELTA1_LOG=info`, `DELTA1_THREADS=min(8, physical)`
* **Numeriek:** `RUSTFLAGS="-C target-cpu=native -C codegen-units=1"`; uitsluitend `f32`.
* **Seed:** globale `42` + per-run nonce in WhyLog.

# 10) Toolingkeuzes (klein, stabiel)

* Crates: `serde`, `serde_json`, `thiserror`, `blake3`, `ring` (Ed25519), `smartcore`, `candle-core` (optioneel), `rayon` (pool), **geen** heavy frameworks.
* **Build security:** reproducible builds + **CycloneDX SBOM** (`cargo sbom`), artefact-signing (Ed25519).

# 11) CI/CD-gates

1. **Model Card + Datasheet verplicht** (JSON).
2. **Fairness-tests** geslaagd.
3. **WhyLog-coverage ≥ 99%** van inferenties in tests.
4. **Reproducible hash** van `cdylib` matcht.

# 12) Logging

* JSON-logging naar stdout; velden: `ts, level, code, request_id, model_id, latency_ms`.
* Geen payloads; alleen hashes en meetwaarden.

# 13) Retentie

* Ruwe inputs: **24u**.
* Features/metrics: **30 dagen**.
* Audit ledger & Model Cards/Datasheets: **7 jaar** (compliance).

# 14) Performance-plafonds (SLO)

* P50 inferentie: **< 15 ms** (tabular), **< 40 ms** (MiniLM Q4) op 1 vCPU.
* P99 **< 150 ms**; foutpercentage **< 0.1%**.

# 15) Human-in-the-loop

* `RiskLevel::High` ⇒ JSON-antwoord `"status":"needs_human_review"` + audit entry; geen automatische beslissing.

# 16) Voorbeeld Model Card (JSON, minimaal)

```json
{
  "model_id": "tabular-logreg-v1",
  "purpose": "fraud_screening",
  "data": {"source":"dataset:abc123","period":"2024-01..2024-06"},
  "metrics": {"auc":0.87,"accuracy":0.81},
  "fairness": {"delta_tpr":0.03,"delta_fpr":0.02},
  "risk": "limited",
  "dp": {"enabled":true,"epsilon":3.0,"delta":1e-5},
  "limitations": ["gevoelig voor ontbrekende income_feature"],
  "contact": "ml-oversight@nederai.example"
}
```

---

**Klaar om te bouwen**: met deze keuzes kun je de traits, FFI-signatures, DB-schema’s en CI-gates direct implementeren zonder frameworks, met PDO-named params en EU-waarden afdwingbaar in code.
