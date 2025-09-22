//! C-compatible API exposed to the PHP interface.
//!
//! Matches the FFI contract defined in `docs/model-design.md`: string-returning
//! version function, explicit status codes (`DeltaCode`) and deterministic
//! routing behaviour that can be audited from the PHP layer.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::OnceLock;

use crate::common::error::{DeltaCode, DeltaError};
use crate::core_data_ingest;
use crate::core_infer_with_ctx;
use crate::core_load_model;
use crate::core_train;
use crate::data::domain::DatasetId;
use crate::export_datasheet;
use crate::export_model_card;
use crate::register_active_model;
use crate::training::domain::{ModelId, VersionName};

static API_VERSION: OnceLock<CString> = OnceLock::new();

#[no_mangle]
pub extern "C" fn delta1_api_version() -> *const c_char {
    API_VERSION
        .get_or_init(|| CString::new("1.0.0").expect("static version string"))
        .as_ptr()
}

#[no_mangle]
pub extern "C" fn delta1_data_ingest(
    filepath: *const c_char,
    out_dataset_id: *mut *const c_char,
) -> i32 {
    if filepath.is_null() || out_dataset_id.is_null() {
        return DeltaCode::InvalidInput as i32;
    }

    let path = unsafe { CStr::from_ptr(filepath) }
        .to_string_lossy()
        .to_string();

    match core_data_ingest(&path, "{}") {
        Ok(dataset_id) => match assign_out_string(out_dataset_id, dataset_id.into_inner()) {
            Ok(_) => DeltaCode::Ok as i32,
            Err(err) => err.code as i32,
        },
        Err(err) => err.code as i32,
    }
}

#[no_mangle]
pub extern "C" fn delta1_train(
    dataset_id: *const c_char,
    train_cfg_json: *const c_char,
    out_model_id: *mut *const c_char,
) -> i32 {
    if dataset_id.is_null() || train_cfg_json.is_null() || out_model_id.is_null() {
        return DeltaCode::InvalidInput as i32;
    }

    let dataset = unsafe { CStr::from_ptr(dataset_id) }
        .to_string_lossy()
        .to_string();
    let cfg = unsafe { CStr::from_ptr(train_cfg_json) }
        .to_string_lossy()
        .to_string();

    let dataset = DatasetId::new(dataset);

    match core_train(dataset, &cfg) {
        Ok(model) => match assign_out_string(out_model_id, model.id.into_inner()) {
            Ok(_) => DeltaCode::Ok as i32,
            Err(err) => err.code as i32,
        },
        Err(err) => err.code as i32,
    }
}

#[no_mangle]
pub extern "C" fn delta1_load_model(model_id: *const c_char, version: *const c_char) -> i32 {
    if model_id.is_null() {
        return DeltaCode::InvalidInput as i32;
    }

    let model_id = unsafe { CStr::from_ptr(model_id) }
        .to_string_lossy()
        .to_string();
    let version = if version.is_null() {
        None
    } else {
        let raw = unsafe { CStr::from_ptr(version) }
            .to_string_lossy()
            .to_string();
        if raw.is_empty() || raw == "latest" {
            None
        } else {
            Some(VersionName::new(raw))
        }
    };

    let model_id = ModelId::new(model_id);
    match core_load_model(&model_id, version.as_ref()) {
        Ok(model) => {
            register_active_model(model);
            DeltaCode::Ok as i32
        }
        Err(err) => err.code as i32,
    }
}

#[no_mangle]
pub extern "C" fn delta1_infer_with_ctx(
    purpose_id: *const c_char,
    subject_id: *const c_char,
    input_json: *const c_char,
) -> *const c_char {
    if purpose_id.is_null() || subject_id.is_null() || input_json.is_null() {
        return error_json(DeltaError::invalid("ffi_null"));
    }

    let purpose = unsafe { CStr::from_ptr(purpose_id) }
        .to_string_lossy()
        .to_string();
    let subject = unsafe { CStr::from_ptr(subject_id) }
        .to_string_lossy()
        .to_string();
    let input = unsafe { CStr::from_ptr(input_json) }
        .to_string_lossy()
        .to_string();

    match core_infer_with_ctx(&purpose, &subject, &input) {
        Ok(prediction) => string_to_raw(prediction.json),
        Err(err) => error_json(err),
    }
}

#[no_mangle]
pub extern "C" fn delta1_export_model_card(model_id: *const c_char) -> *const c_char {
    if model_id.is_null() {
        return error_json(DeltaError::invalid("ffi_null"));
    }

    let model = unsafe { CStr::from_ptr(model_id) }
        .to_string_lossy()
        .to_string();
    let model = ModelId::new(model);

    match export_model_card(&model) {
        Ok(card) => string_to_raw(card),
        Err(err) => error_json(err),
    }
}

#[no_mangle]
pub extern "C" fn delta1_export_datasheet(dataset_id: *const c_char) -> *const c_char {
    if dataset_id.is_null() {
        return error_json(DeltaError::invalid("ffi_null"));
    }

    let dataset = unsafe { CStr::from_ptr(dataset_id) }
        .to_string_lossy()
        .to_string();
    let dataset = DatasetId::new(dataset);

    match export_datasheet(&dataset) {
        Ok(sheet) => string_to_raw(sheet),
        Err(err) => error_json(err),
    }
}

#[no_mangle]
pub extern "C" fn delta1_free_str(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr as *mut c_char);
    }
}

fn assign_out_string(target: *mut *const c_char, value: String) -> Result<(), DeltaError> {
    let cstr = CString::new(value).map_err(|_| DeltaError::internal("ffi_nul_byte"))?;
    unsafe {
        *target = cstr.into_raw();
    }
    Ok(())
}

fn string_to_raw(value: String) -> *const c_char {
    CString::new(value)
        .map(|c| c.into_raw() as *const c_char)
        .unwrap_or_else(|_| fallback_json_raw())
}

fn error_json(err: DeltaError) -> *const c_char {
    let body = format!(
        "{{\"ok\":false,\"code\":{},\"msg\":\"{}\"}}",
        err.code as u32, err.msg
    );
    string_to_raw(body)
}

fn fallback_json_raw() -> *const c_char {
    CString::new("{\"ok\":false}\n".to_string())
        .expect("static fallback is valid")
        .into_raw()
}
