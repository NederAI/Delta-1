# Delta 1 — PHP ↔ Rust via FFI

> Handleiding voor het koppelen van de Rust `cdylib` aan PHP 7.4+ zonder
> frameworks. Beschrijft de actuele C-ABI (`delta1_*`) zoals geïmplementeerd in
> `rust-core/src/api/ffi.rs`.

---

## 1) Prereqs

* **PHP ≥ 7.4** met FFI-extensie.
* **Rust** (stable toolchain) + `cargo`.
* OS: Linux (`.so`), macOS (`.dylib`), Windows (`.dll`).

**php.ini (aanbevolen)**

* **Development/CLI**

  ```ini
  ffi.enable = true
  ```

* **Production (FPM/CLI met preload)** – verklein het aanvalsvlak:

  ```ini
  ffi.enable = preload
  opcache.preload = /var/www/preload.php
  ```

  > In productie FFI-definities preloaden via `FFI::load()` in plaats van
  > dynamisch `FFI::cdef()`.

---

## 2) Rust: cdylib bouwen

**Cargo.toml**

```toml
[package]
name = "delta1"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
```

De relevante exports bevinden zich in `rust-core/src/api/ffi.rs`. Kernpunten:

```rust
#[no_mangle]
pub extern "C" fn delta1_api_version() -> *const c_char { /* OnceLock<CString> */ }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(
    filepath: *const c_char,
    out_dataset_id: *mut *const c_char,
) -> i32 { /* DeltaCode::Ok bij succes */ }

#[no_mangle]
pub extern "C" fn delta1_train(
    dataset_id: *const c_char,
    train_cfg_json: *const c_char,
    out_model_id: *mut *const c_char,
) -> i32 { /* DeltaCode */ }

#[no_mangle]
pub extern "C" fn delta1_load_model(
    model_id: *const c_char,
    version: *const c_char,
) -> i32 { /* registreert actief model */ }

#[no_mangle]
pub extern "C" fn delta1_infer_with_ctx(
    purpose_id: *const c_char,
    subject_id: *const c_char,
    input_json: *const c_char,
) -> *const c_char { /* JSON inclusief whylog_hash */ }

#[no_mangle]
pub extern "C" fn delta1_export_model_card(model_id: *const c_char) -> *const c_char;
#[no_mangle]
pub extern "C" fn delta1_export_datasheet(dataset_id: *const c_char) -> *const c_char;
#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char);
```

Alle functies die strings teruggeven gebruiken heap-gealloceerde `CString`s.
Callers **moeten** `delta1_free_str` aanroepen, behalve voor `delta1_api_version`
(dat is een pointer naar een intern beheerde `CString`).

**Build**

```bash
cd rust-core
cargo build --release
# Resultaat (Linux): target/release/libdelta1.so
```

---

## 3) PHP: laden & wrappers

**Bootstrap (`php-interface/src/bootstrap.php`)**

```php
<?php
declare(strict_types=1);

const DELTA1_LIB = __DIR__ . '/../../rust-core/target/release/libdelta1.so';

$CDEF = <<<CDEF
const char* delta1_api_version(void);
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
CDEF;

$ffi = FFI::cdef($CDEF, DELTA1_LIB);
$GLOBALS['ffi'] = $ffi; // simpele globale voor voorbeelden hieronder

final class DeltaCode
{
    public const Ok = 0;
    public const NoConsent = 1;
    public const PolicyDenied = 2;
    public const ModelMissing = 3;
    public const InvalidInput = 4;
    public const Internal = 5;
}

function delta1_api_version(): string
{
    return FFI::string($GLOBALS['ffi']->delta1_api_version());
}

function delta1_data_ingest(string $path, string $schemaJson = '{}'): array
{
    $out = FFI::new('char*');
    $code = $GLOBALS['ffi']->delta1_data_ingest($path, FFI::addr($out));
    if ($code !== DeltaCode::Ok) {
        return [$code, null];
    }

    try {
        return [$code, FFI::string($out[0])];
    } finally {
        $GLOBALS['ffi']->delta1_free_str($out[0]);
    }
}

function delta1_train(string $datasetId, string $cfgJson): array
{
    $out = FFI::new('char*');
    $code = $GLOBALS['ffi']->delta1_train($datasetId, $cfgJson, FFI::addr($out));
    if ($code !== DeltaCode::Ok) {
        return [$code, null];
    }

    try {
        return [$code, FFI::string($out[0])];
    } finally {
        $GLOBALS['ffi']->delta1_free_str($out[0]);
    }
}

function delta1_load_model(string $modelId, ?string $version = null): int
{
    // Lege string of "latest" ⇒ meest recente versie volgens Rust-implementatie.
    $versionArg = $version ?? '';
    return $GLOBALS['ffi']->delta1_load_model($modelId, $versionArg);
}

function delta1_infer_with_ctx(string $purpose, string $subject, string $payload): string
{
    $ptr = $GLOBALS['ffi']->delta1_infer_with_ctx($purpose, $subject, $payload);
    try {
        return FFI::string($ptr);
    } finally {
        $GLOBALS['ffi']->delta1_free_str($ptr);
    }
}

function delta1_export_model_card(string $modelId): string
{
    $ptr = $GLOBALS['ffi']->delta1_export_model_card($modelId);
    try {
        return FFI::string($ptr);
    } finally {
        $GLOBALS['ffi']->delta1_free_str($ptr);
    }
}

function delta1_export_datasheet(string $datasetId): string
{
    $ptr = $GLOBALS['ffi']->delta1_export_datasheet($datasetId);
    try {
        return FFI::string($ptr);
    } finally {
        $GLOBALS['ffi']->delta1_free_str($ptr);
    }
}
```

> Tip: map `DeltaCode` naar HTTP-status (`0 → 200`, `1 → 403`, `2 → 422/403`,
> `3 → 404`, `4 → 400`, `5 → 500`) in je controllerlaag.

---

## 4) Productie-header & preload

**`delta1.h`**

```c
#define FFI_SCOPE "delta1"
#define FFI_LIB   "libdelta1.so" // pas aan per OS

const char* delta1_api_version(void);
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

**`/var/www/preload.php`**

```php
<?php
FFI::load(__DIR__ . '/delta1.h');
```

Met deze preload-aanpak kan runtimecode simpelweg `FFI::scope('delta1')` gebruiken
en worden de prototypes niet dynamisch geïnterpreteerd.

---

## 5) Foutafhandeling & hygiene

* Controleer altijd de `DeltaCode`-returnwaarde voordat je `FFI::string()` aanroept.
* Roep **altijd** `delta1_free_str` aan op pointers die uit Rust terugkomen.
* Log en audit consent/policy-fouten (`DeltaCode::NoConsent/PolicyDenied`).
* `delta1_api_version()` hoeft niet vrijgegeven te worden.

