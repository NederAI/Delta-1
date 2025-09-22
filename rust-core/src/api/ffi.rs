//! C-compatible API exposed to the PHP interface.
//!
//! TODO: Audit safety of all pointer conversions and document ownership rules explicitly.
//! TODO: Provide structured error reporting instead of sentinel values.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::data::service;
use crate::inference::service as inference_service;
use crate::training::domain::ModelId;
use crate::training::service as training_service;

/// ABI version to coordinate with the PHP layer.
#[no_mangle]
pub extern "C" fn delta1_api_version() -> u32 {
    1
}

/// Ingest a dataset by path and schema definition.
#[no_mangle]
pub extern "C" fn delta1_data_ingest(path: *const c_char, schema: *const c_char) -> u32 {
    if path.is_null() || schema.is_null() {
        // TODO: Surface a dedicated error code instead of zero.
        return 0;
    }

    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .to_string();
    let schema = unsafe { CStr::from_ptr(schema) }
        .to_string_lossy()
        .to_string();

    match service::ingest_file(&path, &schema) {
        Ok(id) => id.raw(),
        Err(err) => {
            let _ = err;
            // TODO: Emit structured logging for ingestion failures.
            0
        }
    }
}

/// Train a model for the provided dataset identifier and config JSON.
#[no_mangle]
pub extern "C" fn delta1_train(dataset_id: u32, cfg_json: *const c_char) -> u32 {
    if cfg_json.is_null() {
        return 0;
    }

    let cfg = unsafe { CStr::from_ptr(cfg_json) }
        .to_string_lossy()
        .to_string();
    let dataset = crate::data::domain::DatasetId::from(dataset_id);

    match training_service::train(dataset, &cfg) {
        Ok(model_id) => model_id.raw(),
        Err(err) => {
            let _ = err;
            // TODO: Bubble up failure reasons through an out-parameter or error buffer.
            0
        }
    }
}

/// Run inference on a model and return a JSON string (caller must free).
#[no_mangle]
pub extern "C" fn delta1_infer(model_id: u32, input: *const c_char) -> *const c_char {
    if input.is_null() {
        return std::ptr::null();
    }

    let model_id = ModelId::new(model_id);
    let input = unsafe { CStr::from_ptr(input) }
        .to_string_lossy()
        .to_string();

    let model = match training_service::load_model(model_id) {
        Ok(model) => model,
        Err(err) => {
            let _ = err;
            // TODO: Capture the error in a thread-local buffer accessible to callers.
            return null_json();
        }
    };

    match inference_service::infer(&model, &input) {
        Ok(prediction) => string_to_raw(prediction.json),
        Err(err) => {
            let _ = err;
            null_json()
        }
    }
}

/// Free strings allocated by Rust.
#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr as *mut c_char);
    }
    // TODO: Investigate pooling returned buffers to reduce churn across FFI boundaries.
}

fn string_to_raw(s: String) -> *const c_char {
    match CString::new(s) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => fallback_json_raw(),
    }
}

fn null_json() -> *const c_char {
    fallback_json_raw()
}

fn fallback_json_raw() -> *const c_char {
    CString::new("{\"ok\":false}".to_string())
        .expect("static fallback json is valid")
        .into_raw()
}

// TODO: Provide helper APIs to translate DeltaError codes to human readable messages.
