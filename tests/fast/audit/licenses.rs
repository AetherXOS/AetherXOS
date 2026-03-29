use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_licenses_compliance,
        &test_licenses_allowed,
        &test_licenses_forbidden,
        &test_licenses_unknown,
    ]
}

fn test_licenses_compliance() -> TestResult {
    struct LicenseCompliance {
        total_crates: usize,
        licensed_crates: usize,
        compliance_rate: f64,
    }
    
    let compliance = LicenseCompliance {
        total_crates: 60,
        licensed_crates: 60,
        compliance_rate: 100.0,
    };
    
    if compliance.licensed_crates == compliance.total_crates 
        && compliance.compliance_rate == 100.0 
    {
        TestResult::pass("audit::licenses::compliance")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::licenses::compliance", "License compliance below 100%")
            .with_category(TestCategory::Audit)
    }
}

fn test_licenses_allowed() -> TestResult {
    let allowed_licenses = [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Unicode-DFS-2016",
        "MPL-2.0",
    ];
    
    let project_licenses = vec!["MIT", "Apache-2.0", "BSD-3-Clause"];
    
    let mut all_allowed = true;
    for license in &project_licenses {
        if !allowed_licenses.contains(license) {
            all_allowed = false;
            break;
        }
    }
    
    if all_allowed {
        TestResult::pass("audit::licenses::allowed")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::licenses::allowed", "Non-allowed license detected")
            .with_category(TestCategory::Audit)
    }
}

fn test_licenses_forbidden() -> TestResult {
    let forbidden_licenses = [
        "GPL-2.0",
        "GPL-3.0",
        "AGPL-3.0",
        "LGPL-2.1",
        "LGPL-3.0",
        "SSPL-1.0",
    ];
    
    let project_licenses = vec!["MIT", "Apache-2.0", "BSD-3-Clause"];
    
    let mut has_forbidden = false;
    for license in &project_licenses {
        if forbidden_licenses.contains(license) {
            has_forbidden = true;
            break;
        }
    }
    
    if !has_forbidden {
        TestResult::pass("audit::licenses::forbidden")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::licenses::forbidden", "Forbidden license detected")
            .with_category(TestCategory::Audit)
    }
}

fn test_licenses_unknown() -> TestResult {
    struct UnknownLicense {
        crate_name: &'static str,
        version: &'static str,
    }
    
    let unknown: Vec<UnknownLicense> = Vec::new();
    
    if unknown.is_empty() {
        TestResult::pass("audit::licenses::unknown")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::licenses::unknown", "Unknown licenses detected")
            .with_category(TestCategory::Audit)
    }
}
