# NederAI Delta 1 – AI‑architectuur

Delta 1 is een toekomstbestendige AI‑architectuur waarin de kern bestaat uit een modulaire *Rust*‑monoliet.  De structuur combineert de eenvoud en robuustheid van een monoliet met de flexibiliteit van microservices: **modulaire monolieten** groeperen logische functies in onafhankelijke modules met duidelijke grenzen; dit levert een hoge ontwikkelsnelheid op zonder de complexiteit van gedistribueerde systemen.  Modules zijn losjes gekoppeld en communiceren via een openbaar API, waardoor u later eenvoudig modules kunt extraheren naar afzonderlijke services.  Deployen gebeurt als één eenheid, waardoor de operationele complexiteit laag blijft.

Rust werd gekozen vanwege de nadruk op geheugen‑ en typeschaarheid: het taalontwerp bevat een *ownership‑based resource management* (OBRM) mechanisme dat resources automatisch vrijgeeft en buffer‑overflows voorkomt.  De compiler elimineert null‑ en dangling pointers.  Rust combineert deze veiligheidsmechanismen met de snelheid van C/C++ dankzij zero‑cost abstractions en statische geheugentoewijzing.  De PHP‑interface maakt gebruik van de **Foreign Function Interface (FFI)**; FFI maakt het mogelijk om functies uit een andere taal aan te roepen.  PHP 7.4 introduceerde de `FFI`‑klasse; door Rust als gedeelde bibliotheek (cdylib) te compileren en een C‑stijl header aan PHP te leveren, kunnen Rust‑functies direct in PHP worden gebruikt.

---

## Repositorystructuur

```
Delta-1/
├── rust-core/           # De modulaire monoliet geschreven in Rust
│   ├── Cargo.toml       # Projectmetadata en dependencies
│   └── src/
│       ├── lib.rs       # Centrale orchestrator; definieert publieke API van modules
│       ├── common/      # Algemeen bruikbare types (errors, configuratie, logging)
│       │   ├── mod.rs
│       │   └── ...
│       ├── data/        # Dataverwerving en -voorbewerking
│       │   ├── domain/
│       │   │   ├── entity.rs
│       │   │   ├── service.rs
│       │   │   └── repository.rs
│       │   ├── infrastructure/  # implementaties (bestands‑/stream‑invoer)
│       │   └── mod.rs
│       ├── training/
│       │   ├── domain/
│       │   │   ├── model.rs
│       │   │   ├── trainer.rs
│       │   │   └── repository.rs
│       │   ├── infrastructure/  # bindings naar ML‑bibliotheken of FFI‑calls naar Python
│       │   └── mod.rs
│       ├── inference/
│       │   ├── domain/
│       │   │   ├── inference.rs
│       │   │   └── repository.rs
│       │   ├── infrastructure/
│       │   └── mod.rs
│       ├── evaluation/
│       │   └── ...
│       ├── api/
│       │   ├── ffi.rs       # C‑ABI voor FFI; exporteert functies naar PHP
│       │   └── http.rs      # optionele native HTTP‑interface (zonder frameworks)
│       └── README.md
├── php-interface/
│   ├── composer.json       # Autoloading en afhankelijkheden (geen frameworks)
│   ├── src/
│   │   ├── bootstrap.php   # Initialiseert FFI en Rust‑bibliotheek
│   │   ├── DataService.php # Voorbeeldservice die Rust‑functies aanroept
│   │   └── Database.php    # PDO‑wrapper met named parameters
│   ├── public/
│   │   └── index.php       # HTTP‑endpoint (optioneel, zonder framework)
│   └── README.md
├── docs/
│   ├── architecture.md     # In‑depth architectuurbeschrijving
│   ├── modules.md          # Documentatie van domein‑modules
│   └── php-ffi.md          # Handleiding voor het koppelen van PHP aan Rust
├── tests/                  # Integratie‑ en eenheidstests
│   ├── rust/
│   └── php/
└── docker/
    ├── Dockerfile          # Container build (Rust + PHP + FFI)
    └── entrypoint.sh
```

### Beschrijving van hoofdmappen

* **rust-core/** – bevat de modulaire monoliet. Elk domein heeft een eigen map met `domain`, `infrastructure` en een `mod.rs`.  Modules zijn zo onafhankelijk mogelijk; ze bevatten entiteiten, services en repositories, gebaseerd op *Domain‑Driven Design* (DDD).  Het gebruik van DDD dwingt je tot het definiëren van entiteiten, services, API’s en een repository‑patroon; dat creëert duidelijke grenzen en voorkomt dat logica verspreid raakt over het systeem.

* **api/** – dit exposeert de Rust‑modules naar de buitenwereld.  `ffi.rs` bevat functies met `#[no_mangle]` en `extern "C"` zodat ze in de cdylib worden geëxporteerd en FFI‑compatible zijn.  `http.rs` kan optioneel een lichte HTTP‑interface bieden door gebruik te maken van de standaardbibliotheek (`std::net`), aangezien frameworks niet gewenst zijn.

* **php-interface/** – de PHP‑laag laadt de gedeelde bibliotheek via `FFI::cdef()`.  In `bootstrap.php` wordt de C‑header gedeclareerd en de Rust‑bibliotheek geladen; de class `DataService` wikkelt deze FFI‑aanroepen zodat de rest van de code geen FFI‑details hoeft te kennen.  `Database.php` bevat een eenvoudige PDO‑wrapper met named parameters om data op te slaan.

* **docs/** – documentatiebestanden die de rationale, module‑interfaces en instructies voor het gebruik van FFI uitleggen.

* **tests/** – scheidt Rust‑ en PHP‑tests.  Gebruik `cargo test` voor Rust‑modules en PHPUnit voor PHP‑code; integratietests verifiëren de FFI‑koppeling.

---

## Inhoud van `rust-core/src`

### `lib.rs`

`lib.rs` orkestreert de modules en herexporteert publieke functies.  Het definieert eveneens de *hexagonale architectuur* door de grenzen tussen domeinlogica en infrastructuur scherp te houden.

```rust
pub mod common;
pub mod data;
pub mod training;
pub mod inference;
pub mod evaluation;
pub mod api;

// centraliseer fouttypes
pub use common::error::DeltaError;

// herexporteer publieke functies voor FFI/HTTP
pub use api::ffi::{data_ingest, train_model, run_inference};
```

### Domeinmodules

* **domain/entity.rs** – definieert kernobjecten (bv. `Dataset`, `Model`, `InferenceRequest`) en houdt velden privé (`pub(crate)`) om encapsulatie af te dwingen.

* **domain/service.rs** – bevat business‑logica; modules zoals `trainer` initialiseren, trainen en evalueren modellen zonder frameworks.

* **domain/repository.rs** – beschrijft traits voor opslaginteractie; concrete implementaties zitten in `infrastructure/`.

* **infrastructure/** – implementaties van externe technologie.  Voor dataverzameling kan hier een CSV‑reader staan; voor training eventueel bindings naar een Python‑script via FFI.

### `api/ffi.rs`

Exporteert domeinfuncties via C‑ABI.  Functies gebruiken FFI‑veilige types en retourneren eenvoudige waarden.

```rust
use std::ffi::{CStr, CString};
use crate::data;

#[no_mangle]
pub extern "C" fn data_ingest(path: *const c_char) -> u32 {
    let cstr = unsafe { CStr::from_ptr(path) };
    let filename = cstr.to_str().unwrap_or_default();
    match data::ingest_file(filename) {
        Ok(id) => id,
        Err(_) => 0,
    }
}
```

Het `#[no_mangle]` attribuut voorkomt name mangling zodat PHP de functie correct kan vinden.

---

## PHP‑interface

### `bootstrap.php`

```php
<?php
$ffi = FFI::cdef(
    "unsigned int data_ingest(const char* path);
     unsigned int train_model(unsigned int dataset_id, const char* config);
     const char* run_inference(unsigned int model_id, const char* input);",
    __DIR__ . '/../rust-core/target/release/libdelta1.so'
);

function delta1_ingest(string $path): int {
    return $GLOBALS['ffi']->data_ingest($path);
}
function delta1_train(int $datasetId, string $configJson): int {
    return $GLOBALS['ffi']->train_model($datasetId, $configJson);
}
function delta1_infer(int $modelId, string $input): string {
    $cstr = $GLOBALS['ffi']->run_inference($modelId, $input);
    return FFI::string($cstr);
}
?>
```

FFI maakt het mogelijk functies uit een andere taal aan te roepen, en PHP 7.4 voegde hiervoor de `FFI`‑klasse toe.

### `DataService.php`

```php
<?php
class DataService {
    public function importCsv(string $path): int {
        return delta1_ingest($path);
    }
    public function train(array $config): int {
        $json = json_encode($config);
        return delta1_train($this->datasetId, $json);
    }
    public function infer(string $input): string {
        return delta1_infer($this->modelId, $input);
    }
}
?>
```

### `Database.php`

Een PDO‑wrapper die named parameters gebruikt om SQL‑injecties te voorkomen.

---

## Werkwijze en ontwikkelrichtlijnen

1. **Monolith first** – begin met een modulaire monoliet; microservices voegen complexiteit toe en leveren weinig voordelen tot schaal nodig is.

2. **Definieer duidelijke grenzen** – modules moeten onafhankelijk en inwisselbaar zijn met een goed gedefinieerde interface; is er te veel onderlinge communicatie, heroverweeg dan de grenzen.

3. **Gebruik DDD en hexagonale architectuur** – groepeer code per domein en scheid infrastructuur van de kern.

4. **Rust‑ontwikkeling** – profiteer van Rusts geheugenveiligheid en prestaties; code compileert naar meerdere platforms.

5. **PHP‑FFI interface** – compileer de Rust‑kern als cdylib, exporteer functies met `#[no_mangle] extern "C"` en roep ze aan via `FFI::cdef()`.

6. **Geen frameworks** – gebruik de standaardbibliotheek en minimaliseer afhankelijkheden.  In PHP wordt geen framework gebruikt; een eventuele HTTP‑router wordt handmatig gebouwd.

7. **Tests en CI** – schrijf unit‑ en integratietests voor zowel Rust‑ als PHP‑lagen; gebruik CI om cdylibs te bouwen en tests te draaien.

8. **Uitbreidbaarheid** – modules kunnen later als microservices worden uitgelicht dankzij hun duidelijke grenzen.

---
