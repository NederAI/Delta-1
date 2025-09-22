# Agent Model Realization Roadmap

> Roadmap voor het realiseren van agent-achtige modellen binnen Delta 1. Richt zich op gecontroleerde iteraties, naleving van privacy-/compliance-eisen en naadloze integratie met de bestaande Rust-monoliet en PHP-FFI-laag.

## Scope & uitgangspunten

* **Agent-definitie:** taakgerichte, semi-autonome modellen die plannen, beslissen en externe tools kunnen aanroepen via gecontroleerde interfaces.
* **Kernprincipes:** determinisme waar mogelijk, consent-gedreven dataflow, uitlegbaarheid via WhyLog en reproduceerbare builds (`delta1_*`).
* **Beperkingen:** geen third-party gesloten modellen; alle componenten moeten lokaal train-/hostbaar zijn en voldoen aan GDPR/AI-Act guardrails.

## Faseoverzicht

| Fase | Doel | Belangrijkste deliverables | Gate/criteria |
| --- | --- | --- | --- |
| 0. Intake & governance | Use-case, risico's en compliance-afbakening | Intake canvas, DPIA-lite, purpose-registratie | Privacy/compliance akkoord, `DeltaCode::NoConsent` flows getest |
| 1. Omgevings- & datamodellering | Operationele context en data-stromen modelleren | Interaction map, data inventory, consent lookup design | Dataminimalisatie-check, datasheet skeleton |
| 2. Capaciteitenprototype | Kern-agentgedrag bewijzen met sandbox datasets | Prototype agent loop, tool-API mock, logging-schema | FFI-proof-of-concept, seed-consistente runs |
| 3. Orchestratie & tool-integratie | Tooling, routering en controlelagen harden | Tool registry (Rust), router rules, PHP bindings | Security review, command budget limiter |
| 4. Veiligheid & alignment | Misuse-, bias- en consenttesting | Red-team log, fairness/DP rapport, policy filters | ΔTPR/ΔFPR limieten, policy regression suite |
| 5. Pre-productie validatie | Performance, explainability, observability | WhyLog coverage ≥99%, latency profielen, audit ledger mock | SLO-rapport, audit-trail handtekeningen |
| 6. Release & lifecycle | Productie-naar uitrol en onderhoud | Model card, runbook, monitoring plan | RFC goedgekeurd, version pinning + rollback plan |

## Fase-details

### Fase 0 — Intake & governance

* Verzamel business context, doelmetrieken en menselijke escalatiepaden (HITL) en leg vast in een intake canvas (Markdown) in `docs/`.
* Start een DPIA-lite die zich richt op de nieuwe agent-acties en extra gegevenscategorieën; registreer purpose-id's in consent store.
* Definieer risicoklassen en beslis welke `DeltaCode`-varianten naar de buitenwereld gemapt worden via PHP (HTTP-statusen + audit logging).

### Fase 1 — Omgevings- & datamodellering

* Modelleer interacties tussen agent, externe tools en datasources; documenteer inputs/outputs als JSON-schema's.
* Bepaal welke gegevens als features worden gehasht via `SimpleHash` en welke ruwe velden tijdelijk bewaard mogen blijven (<24u).
* Werk datasheet-templates bij (export via `export_datasheet`) en leg consent lookup-tabellen vast (PDO schema + migratieplan).

### Fase 2 — Capaciteitenprototype

* Bouw een sandbox-runner in Rust (`inference::service`) die een agent-loop simuleert met mocked tools en deterministische seeds.
* Valideer dat logging en WhyLog-hashes voor elke actie consistent zijn en dat audit events in de append-only ledger passen.
* Richt een experimentele routerregel in (`router::SSMRouter`) om agent requests te onderscheiden (bv. `"agent": true`).

### Fase 3 — Orchestratie & tool-integratie

* Introduceer een `ToolRegistry` module (Rust) met trait-gebaseerde adapters; implementeer minimaal een database- en een HTTP-tool.
* Beperk tool-aanroepen via command budgetting en sandboxing; exposeer configuratie via `AppCfg` zodat PHP `bootstrap` dit kan laden.
* Breid de PHP-FFI laag uit met duidelijke wrappers (`DataService`) om agent-contexten en WhyLog-referenties te overdragen.

### Fase 4 — Veiligheid & alignment

* Voer misbruikscenario-tests uit (prompt-injectie, policy bypass) en documenteer mitigaties (regex, allowlists, human review).
* Meet fairness-metrieken per subgroep (ΔTPR/ΔFPR/ΔPPV) op agentbeslissingen; verbind DP-SGD (`TrainConfig.dp`) waar data gevoelig is.
* Automatiseer policy checks in CI: fails bij ontbrekende redactie, ontbrekende consent of overschrijding van risicodrempels.

### Fase 5 — Pre-productie validatie

* Draai prestatietests met realistische workloads; borg P50/P99-latencies en foutpercentages binnen de SLO's uit `model-design.md`.
* Verzeker WhyLog-coverage ≥99% en valideer Merkle-ketens (audit ledger) inclusief Ed25519-signaturen in een staging omgeving.
* Genereer uitlegbaarheidsrapporten (feature-bijdragen, token-saliency) en koppel ze aan de WhyLog-hash voor reproduceerbaarheid.

### Fase 6 — Release & lifecycle

* Publiceer een volledige model card + datasheet; documenteer agent-specifieke risico's en mitigaties.
* Stel een runbook op voor incident response, rollback-procedure en monitoring (metrics, logs, alerts) en archiveer in `docs/ops/` (nieuw).
* Plan lifecycle hooks: retraining cadence, consent-herbevestiging, model archival en periodieke bias audits.

## Documentatie & artefacten

* **Roadmap-tracker:** eenvoudige kanban (Notion/Jira) met fasen als kolommen; link naar relevante Markdown-documenten.
* **Beslissingslog:** Markdown `docs/decisions/{YYYY}-{slug}.md` met architectuurkeuzes en referenties naar audits.
* **Test suites:** uitbreidingen op `tests/rust/` voor agent-loop simulaties en `tests/php/` voor FFI-endpoints.
* **CI-versterking:** voeg checks toe voor WhyLog coverage, fairness rapportage en SBOM verificatie per fase.

## KPI's & monitoring

* **Effectiviteit:** taak-succesratio, gemiddelde beslissingsscore vs. baseline modellen.
* **Veiligheid:** aantal geblokkeerde policy-overtredingen, HITL-escalaties, consent-violations (verwacht 0).
* **Prestatie:** latenties per tool-call, command budget overschrijdingen, resource footprint (CPU/RAM) per agent-episode.
* **Transparantie:** percentage requests met volledige WhyLog + model card updates.

## Risico's & mitigaties

| Risico | Impact | Mitigatie |
| --- | --- | --- |
| Tool-misbruik of privilege-escalatie | Hoge | Sandboxing, allowlists, rate limiting, menselijke review bij `RiskLevel::High`. |
| Dataschending via agent-logs | Hoog | Redactie/normalisatie (NFKC), 24u-retentie, audit ledger encryptie in rust. |
| Onverklaarbare beslissingen | Middel | Verplicht WhyLog + surrogate uitleg, menselijke override, model card updates. |
| Fairness-regressie | Hoog | CI-gates op ΔTPR/ΔFPR, automatische reweighing/post-processing bij overschrijding. |
| Operationele drift | Middel | Monitoring, automatische retraining triggers, drift-analyse via `evaluation::service`. |

---

Met dit roadmapdocument kan het team agent-modellen gecontroleerd en compliant realiseren binnen de bestaande Delta 1-architectuur, zonder de deterministische en auditbare fundamenten van het platform los te laten.
