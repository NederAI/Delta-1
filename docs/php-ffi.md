# Delta 1 — php-ffi.md

> Koppeling tussen PHP (zonder framework) en de Rust-kern via **FFI**. Focus op eenvoud, veiligheid, performance en EU-conforme inzet.

---

## 1) Prereqs

* **PHP ≥ 7.4** met FFI.
* **Rust** (stable toolchain).
* OS: Linux (`.so`), macOS (`.dylib`), Windows (`.dll`).

**php.ini (aanbevolen)**

* **Development (CLI):**

  ```ini
  ffi.enable = true
  ```
* **Production (FPM/Apache):** minimaliseer aanvalsvlak

  ```ini
  ffi.enable = preload
  opcache.preload = /var/www/preload.php
  ; In preload.php: uitsluitend whitelisted headers/libraries laden
  ```

  > In productie vermijd `FFI::cdef()`; gebruik **preload + FFI::load()** met vaste header.

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

[dependencies]
# voeg alleen strikt noodzakelijke crates toe
```

**lib.rs (C-ABI)**

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle] pub extern "C" fn delta1_api_version() -> u32 { 1 }

#[no_mangle]
pub extern "C" fn delta1_data_ingest(path: *const c_char, schema: *const c_char) -> u32 {
    let path   = unsafe { CStr::from_ptr(path)   }.to_string_lossy();
    let schema = unsafe { CStr::from_ptr(schema) }.to_string_lossy();
    // TODO: call into data::ingest_file(&path, &schema)
    1 /* dataset_id > 0 on success, 0 on failure */
}

#[no_mangle]
pub extern "C" fn delta1_train(dataset_id: u32, cfg: *const c_char) -> u32 {
    let _cfg = unsafe { CStr::from_ptr(cfg) }.to_string_lossy();
    // TODO: call training::train(...)
    1 /* model_id */
}

#[no_mangle]
pub extern "C" fn delta1_infer(model_id: u32, input: *const c_char) -> *const c_char {
    let _input = unsafe { CStr::from_ptr(input) }.to_string_lossy();
    // TODO: call inference::infer(...)-> JSON string
    let out = CString::new(r#"{"ok":true}"#).unwrap();
    out.into_raw() // caller moet vrijgeven
}

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char) {
    if ptr.is_null() { return; }
    unsafe { let _ = CString::from_raw(ptr as *mut c_char); }
}
```

**Build**

```bash
cargo build --release
# Target:
#   Linux:  target/release/libdelta1.so
#   macOS:  target/release/libdelta1.dylib
#   Win:    target/release/delta1.dll
```

---

## 3) PHP: laden & wrappers

**Bestand:** `php-interface/src/bootstrap.php`

```php
<?php
declare(strict_types=1);

/**
 * Resolve platform-specifieke bestandsnaam.
 */
function delta1_lib_path(): string {
    $base = realpath(__DIR__ . "/../../rust-core/target/release");
    $isWin = PHP_OS_FAMILY === 'Windows';
    $isMac = PHP_OS_FAMILY === 'Darwin';
    if ($isWin) return $base . DIRECTORY_SEPARATOR . "delta1.dll";
    if ($isMac) return $base . DIRECTORY_SEPARATOR . "libdelta1.dylib";
    return $base . DIRECTORY_SEPARATOR . "libdelta1.so";
}

/**
 * C-prototypes (alleen in dev/CLI; in prod via FFI::load met header).
 */
$CDEF = <<<CDEF
unsigned int delta1_api_version(void);
unsigned int delta1_data_ingest(const char* path, const char* schema_json);
unsigned int delta1_train(unsigned int dataset_id, const char* config_json);
const char*  delta1_infer(unsigned int model_id, const char* input_json);
void         delta1_free_str(const char* ptr);
CDEF;

$lib = delta1_lib_path();

/**
 * In productie: preload + FFI::load('delta1.h') i.p.v. cdef.
 */
$ffi = FFI::cdef($CDEF, $lib);

/** Light wrappers met veilige string-afhandeling **/
function delta1_api_version(): int {
    return $GLOBALS['ffi']->delta1_api_version();
}

function delta1_data_ingest(string $path, string $schemaJson): int {
    return $GLOBALS['ffi']->delta1_data_ingest($path, $schemaJson);
}

function delta1_train(int $datasetId, string $configJson): int {
    return $GLOBALS['ffi']->delta1_train($datasetId, $configJson);
}

function delta1_infer(int $modelId, string $inputJson): string {
    $ptr = $GLOBALS['ffi']->delta1_infer($modelId, $inputJson);
    try { return FFI::string($ptr); }
    finally { $GLOBALS['ffi']->delta1_free_str($ptr); }
}
```

**Voorbeeldgebruik (CLI)**

```php
<?php
require __DIR__.'/src/bootstrap.php';

echo "API v".delta1_api_version().PHP_EOL;
$ds = delta1_data_ingest(__DIR__.'/data.csv', '{"type":"csv"}');
$m  = delta1_train($ds, '{"lr":0.01,"epochs":5}');
$out = delta1_infer($m, '{"x":[1,2,3]}');
echo $out.PHP_EOL;
```

---

## 4) Productie-header & preload (veilig)

**`delta1.h`**

```c
#define FFI_SCOPE "delta1"
#define FFI_LIB   "libdelta1.so" /* pas aan per OS of absolute path */

unsigned int delta1_api_version(void);
unsigned int delta1_data_ingest(const char* path, const char* schema_json);
unsigned int delta1_train(unsigned int dataset_id, const char* config_json);
const char*  delta1_infer(unsigned int model_id, const char* input_json);
void         delta1_free_str(const char* ptr);
```

**`/var/www/preload.php`**

```php
<?php
FFI::load(__DIR__ . '/delta1.h'); // preload vaste header
```

**php.ini (prod)**

```ini
ffi.enable = preload
opcache.preload = /var/www/preload.php
```

> Resultaat: geen runtime `FFI::cdef()`; alleen vooraf goedgekeurde symbolen.

---

## 5) PDO (named parameters) voor metadata

**`php-interface/src/Database.php`**

```php
<?php
final class Database {
    private \PDO $pdo;
    public function __construct(string $dsn, string $user, string $pass) {
        $this->pdo = new \PDO($dsn, $user, $pass, [
            \PDO::ATTR_ERRMODE => \PDO::ERRMODE_EXCEPTION,
            \PDO::ATTR_DEFAULT_FETCH_MODE => \PDO::FETCH_ASSOC,
        ]);
    }

    public function insertModel(int $modelId, string $version, string $metricsJson): void {
        $sql = "INSERT INTO models(id, version, metrics_json) VALUES(:id,:v,:m)";
        $stmt = $this->pdo->prepare($sql);
        $stmt->execute([':id'=>$modelId, ':v'=>$version, ':m'=>$metricsJson]);
    }
}
```

---

## 6) Memory & ownership

* Strings uit Rust worden gealloceerd met `CString::into_raw()`.
* **Altijd** vrijgeven in PHP via `delta1_free_str($ptr)`.
* Geen pointers cachen over request-grenzen.
* Houd cdylib **stateless** of bescherm gedeelde staat met `Mutex`/`RwLock`.

---

## 7) Fouten & codes

Conventie (simple):

* `0` ⇒ fout; `>0` ⇒ OK id.
* String-API’s retourneren JSON met `{"ok":false,"code":50,"msg":"..."}`.

Uitbreiding (optioneel):

```c
/* Haal laatste fout op als JSON, thread-local in Rust */
const char* delta1_last_error(void);
```

---

## 8) Beveiliging

* Minimaliseer export: alleen noodzakelijke symbolen.
* Valideer **alle** input (pad, JSON schema).
* Hardened build (strip symbols, LTO waar mogelijk).
* Prod: `ffi.enable=preload`, vaste pad/naam library, geen `cdef`.
* Scheid gebruikersrechten; FPM gebruiker heeft alleen leesrecht op cdylib.

---

## 9) Performance-tips

* Prefer **batch infer** paden (JSON array in één call).
* Vermijd onnodige allocaties; hergebruik buffers in Rust.
* Gebruik vaste seeds en pinned toolchains voor reproduceerbaarheid.
* Warm-up stap (lazy init) per proces.

---

## 10) Testen

* **Unit (Rust):** `cargo test` per module.
* **Contract (PHP→FFI):** controleer symbolen + prototypes (dev: `FFI::scope`).
* **Integratie:** ingest→train→infer→assert JSON schema.
* **Leak checks:** in tests altijd `delta1_free_str()` aanroepen.

---

## 11) Deploy & runtime

* Bibliotheek pad bekend maken:

  * Linux: `LD_LIBRARY_PATH` of absoluut pad.
  * macOS: `DYLD_LIBRARY_PATH`.
  * Windows: map met `delta1.dll` in `PATH` of zelfde dir als PHP bin.
* Eén artefact per release; versies via bestandsnaam of symlink `current`.

---

## 12) Troubleshooting

| Symptoom                         | Oorzaak                     | Fix                                 |
| -------------------------------- | --------------------------- | ----------------------------------- |
| `FFI\ParserException`            | Header/typedef fout         | Corrigeer C-prototypes; puntkomma’s |
| `undefined symbol: delta1_infer` | Naam-mangling of andere lib | `#[no_mangle]`, juiste lib laden    |
| Segfault bij infer               | Ptr niet vrijgegeven        | Altijd `delta1_free_str()`          |
| “Cannot load library”            | Pad/rechten/loader-var      | Absoluut pad, env var, rechten      |
| Hang/lock                        | Globale mutable state       | Vermijd; gebruik `Mutex/RwLock`     |

---

## 13) Minimale integratieflow (PHP)

```php
$ds = delta1_data_ingest('/data/input.csv', '{"type":"csv"}');
if ($ds === 0) { /* afhandelen */ }

$model = delta1_train($ds, '{"lr":0.01,"epochs":3}');
$out   = delta1_infer($model, '{"x":[1,2,3]}');
$json  = json_decode($out, true, flags: JSON_THROW_ON_ERROR);
```
