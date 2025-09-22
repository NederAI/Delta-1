# NederAI Delta 1 – AI-architectuur

Delta 1 is een toekomstbestendige AI-architectuur waarin de kern bestaat uit een modulaire Rust-monoliet. De structuur combineert de eenvoud en robuustheid van een monoliet met de flexibiliteit van microservices: modules zijn losjes gekoppeld, communiceren via duidelijke grenzen en kunnen later worden geëxtraheerd naar afzonderlijke services. Deployments blijven daardoor eenvoudig, terwijl ontwikkelteams domeinmodules autonoom kunnen evolueren. De architectuur is geworteld in Europese waarden zoals privacy-by-design, transparantie, menselijke controle en non-discriminatie, zodat implementaties aantoonbaar in lijn met GDPR en de aankomende AI-wetgeving blijven.

Rust is gekozen omwille van geheugen- en typestrictheid. Ownership-gebaseerd resourcemanagement voorkomt buffer-overflows en garandeert deterministische vrijgave van resources. De compiler elimineert null- en dangling pointers en levert C/C++-prestaties dankzij zero-cost abstractions en statische geheugentoewijzing. De PHP-interface gebruikt de Foreign Function Interface (FFI) die sinds PHP 7.4 beschikbaar is: de Rust-kerntoepassing wordt als gedeelde bibliotheek (`cdylib`) gecompileerd en via een C-header beschikbaar gemaakt, zodat PHP-functies rechtstreeks de Rust-API kunnen aanspreken.

## Kernarchitectuur van Delta 1

### Modulaire monoliet in Rust (`rust-core/`)

De map `rust-core/` bevat de centrale crate die alle domeinen bundelt. De crate wordt als één bibliotheek gebouwd en exporteert stabiele functies voor FFI-consumenten.

#### Orchestratie (`src/lib.rs`)

* Declareert de modules `api`, `common`, `data`, `evaluation`, `inference` en `training`.
* Re-exporteert de belangrijkste servicefuncties (`data::service::ingest_file`, `training::service::{train, load_model}`, `inference::service::infer`) zodat interne consumers één façade kunnen gebruiken.
* Vormt het startpunt voor het opzetten van repositories en configuratie wanneer de bootstrap-sequentie wordt toegevoegd.

#### Gemeenschappelijke bouwstenen (`src/common/`)

* `config.rs` laadt runtimeconfiguratie zoals `DELTA1_DATA_ROOT`, regio en logniveau.
* `error.rs` definieert het `DeltaError`/`DeltaResult`-model met stabiele `DeltaCode`-waarden die over de FFI-grens kunnen.
* `ids.rs` bevat de deterministische `SimpleHash` waarmee dataset- en modelidentifiers worden afgeleid.
* `time.rs` en `log.rs` leveren respectievelijk tijd- en logginghulpmiddelen met JSON-logging.
* `buf.rs` biedt een herbruikbare bytebuffer voor IO-intensieve paden.

#### Domeinen

* **`data/`** – beheert datasetmetadata.
  * `domain.rs` beschrijft `Dataset`, `DatasetId` en de `DataRepo`-trait.
  * `service.rs` voert bestandsingestie uit: leest regels, normaliseert/hasht ze met `SimpleHash` en bouwt een `DatasetId`.
  * `repo_fs.rs` implementeert een bestandenopslag op basis van `AppCfg::data_root` (met TODO’s voor vollediger metadataherstel).
* **`training/`** – verzorgt modelversies en artefacten.
  * `domain.rs` definieert `ModelId`, `ModelVersion`, `TrainConfig` en de `ModelRepo`/`Trainer`-interfaces.
  * `service.rs` combineert dataset-id en configuratie om een deterministische `ModelId` te berekenen; `load_model` is de plaats waar het ophalen van artefacten wordt aangesloten.
  * `repo_fs.rs` schrijft artefact-headers weg in het `models/`-pad en vormt de basis voor versiebeheer.
* **`inference/`** – levert synchron en batch-inferentie.
  * `domain.rs` modelleert `Prediction` en de `InferEngine`-trait.
  * `service.rs` verwerkt verzoeken, meet latency (`time::now_ms`) en construeert JSON-antwoordpayloads.
  * `workers.rs` introduceert een lichte threadpool (`Pool`) voor CPU-intensieve taken.
* **`evaluation/`** – groepeert evaluatie- en driftfunctionaliteit.
  * `domain.rs` definieert `EvalSuite`, `DriftStats` en de `EvalRepo`-trait.
  * `service.rs` bevat placeholders voor het genereren van evaluatiekaarten en driftstatistieken.

#### Publieke API (`src/api/`)

* `mod.rs` groepeert publiek beschikbare bindingen.
* `ffi.rs` exporteert de functies `delta1_api_version`, `delta1_data_ingest`, `delta1_train`, `delta1_infer` en `delta1_free_str` met een C-ABI. Pointers worden zorgvuldig gecontroleerd op null en resultaten worden vertaald naar simpele retourcodes of JSON.

#### Gegevens- en modelstroom

1. **Dataset-ingestie** – `delta1_data_ingest` roept `data::service::ingest_file` aan, genereert een hash-gebaseerde `DatasetId` en (TODO) persisteert metadata via `DataRepo`.
2. **Training** – `delta1_train` zet de dataset-id om naar `DatasetId` en produceert via `training::service::train` een deterministische `ModelId`; persistente opslag volgt via `ModelRepo` zodra geïmplementeerd.
3. **Inferentie** – `delta1_infer` laadt (TODO) het model via `training::service::load_model` en verwerkt voorspellingen in `inference::service::infer`, dat latencies meet en een JSON-resultaat terugstuurt.
4. **Evaluatie** – `evaluation::service::evaluate` en `drift` vormen de basis voor kwaliteits- en driftmonitoring op `ModelVersion`-niveau.

### PHP-FFI laag (`php-interface/`)

De PHP-laag vormt een dunne schil rond de Rust-bibliotheek:

* `src/bootstrap.php` declareert de C-header via `FFI::cdef()` en laadt `libdelta1.so`.
* `src/DataService.php` biedt een objectgeoriënteerde façade die de FFI-aanroepen verpakt zodat applicatiecode geen pointers hoeft te beheren.
* `src/Database.php` levert een lichte PDO-wrapper met named parameters voor opslag.
* `public/index.php` demonstreert hoe HTTP-endpoints (zonder frameworks) direct de services kunnen aanroepen.
* `composer.json` beschrijft autoloading en blijft afhankelijkheidsvrij.

## Ondersteunende infrastructuur

* **`docs/`** – verdiepende documentatie:
  * `architecture.md` voor de algemene ontwerpprincipes,
  * `modules.md` met moduleoverzichten,
  * `agent-model-roadmap.md` als roadmap voor agent-modellen en governance,
  * `php-ffi.md` als handleiding voor het koppelen van PHP aan Rust,
  * `rust-core.md` met aanvullende crate-details.
* **`rust-core/README.md`** – verduidelijkt crate-specifieke richtlijnen en buildinstructies.
* **`tests/`** – placeholdermappen voor Rust- (`cargo test`) en PHP-tests (PHPUnit) die de FFI-koppeling integreren.
* **`docker/`** – levert een Dockerfile en entrypoint die Rust, PHP en de FFI-opzet in één container bundelen.

## Repositorystructuur

```
Delta-1/
├── README.md
├── docker/
│   ├── Dockerfile
│   └── entrypoint.sh
├── docs/
│   ├── architecture.md
│   ├── modules.md
│   ├── php-ffi.md
│   └── rust-core.md
├── php-interface/
│   ├── README.md
│   ├── composer.json
│   ├── public/
│   │   └── index.php
│   └── src/
│       ├── DataService.php
│       ├── Database.php
│       └── bootstrap.php
├── rust-core/
│   ├── Cargo.toml
│   ├── README.md
│   └── src/
│       ├── api/
│       │   ├── ffi.rs
│       │   └── mod.rs
│       ├── common/
│       │   ├── buf.rs
│       │   ├── config.rs
│       │   ├── error.rs
│       │   ├── ids.rs
│       │   ├── log.rs
│       │   └── time.rs
│       ├── data/
│       │   ├── domain.rs
│       │   ├── mod.rs
│       │   ├── repo_fs.rs
│       │   └── service.rs
│       ├── evaluation/
│       │   ├── domain.rs
│       │   ├── mod.rs
│       │   └── service.rs
│       ├── inference/
│       │   ├── domain.rs
│       │   ├── mod.rs
│       │   ├── service.rs
│       │   └── workers.rs
│       ├── training/
│       │   ├── domain.rs
│       │   ├── mod.rs
│       │   ├── repo_fs.rs
│       │   └── service.rs
│       └── lib.rs
└── tests/
    ├── php/
    │   └── placeholder.txt
    └── rust/
        └── placeholder.txt
```

Deze structuur houdt de kernarchitectuur scherp in beeld: de Rust-crate levert duidelijk afgebakende domeinen, de PHP-laag fungeert als FFI-bridge en ondersteunende mappen documenteren, testen en containeriseren het geheel. Hierdoor blijft Delta 1 uitbreidbaar zonder de operationele eenvoud van een monoliet te verliezen.
