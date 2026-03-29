pub const BACKEND_NONE: &str = "none";
pub const BACKEND_VMX_DETECTED: &str = "vmx:detected";
pub const BACKEND_VMX_ENABLED: &str = "vmx:enabled";
pub const BACKEND_VMX_ACTIVE: &str = "vmx:vmxon+vmcs";
pub const BACKEND_SVM_DETECTED: &str = "svm:detected";
pub const BACKEND_SVM_ENABLED: &str = "svm:enabled";
pub const BACKEND_SVM_ACTIVE: &str = "svm:enabled+vmcb";
pub const BACKEND_EL2_DETECTED: &str = "el2:detected";
pub const BACKEND_EL2_ACTIVE: &str = "el2:active";
pub const BACKEND_EL2_FULL: &str = "el2:active+gic+timer";

pub const BACKEND_FAMILY_NONE: &str = "none";
pub const BACKEND_FAMILY_VMX: &str = "vmx";
pub const BACKEND_FAMILY_SVM: &str = "svm";
pub const BACKEND_FAMILY_EL2: &str = "el2";

pub const BACKEND_MODE_FULL: &str = "backend-full";
pub const BACKEND_MODE_BASIC: &str = "backend-basic";
pub const BACKEND_MODE_BLOCKED: &str = "backend-blocked";
pub const BACKEND_MODE_NONE: &str = "backend-none";

#[inline(always)]
pub fn backend_family(backend_detail: &'static str) -> &'static str {
    if backend_detail.starts_with("vmx:") {
        BACKEND_FAMILY_VMX
    } else if backend_detail.starts_with("svm:") {
        BACKEND_FAMILY_SVM
    } else if backend_detail.starts_with("el2:") {
        BACKEND_FAMILY_EL2
    } else {
        BACKEND_FAMILY_NONE
    }
}

#[inline(always)]
pub fn backend_mode_from_class(
    backend_detail: &'static str,
    operation_class: &'static str,
) -> &'static str {
    if backend_detail == BACKEND_NONE {
        BACKEND_MODE_NONE
    } else {
        match operation_class {
            super::runtime::OPERATION_CLASS_FULL => BACKEND_MODE_FULL,
            super::runtime::OPERATION_CLASS_BASIC => BACKEND_MODE_BASIC,
            super::runtime::OPERATION_CLASS_BLOCKED => BACKEND_MODE_BLOCKED,
            _ => BACKEND_MODE_NONE,
        }
    }
}

#[inline(always)]
pub fn feature_backend_mode(
    feature_ready: bool,
    primary_scope: &'static str,
    secondary_scope: Option<&'static str>,
) -> &'static str {
    if !feature_ready {
        BACKEND_MODE_BLOCKED
    } else if primary_scope == "fully-enabled"
        && secondary_scope.unwrap_or("fully-enabled") == "fully-enabled"
    {
        BACKEND_MODE_FULL
    } else {
        BACKEND_MODE_BASIC
    }
}
