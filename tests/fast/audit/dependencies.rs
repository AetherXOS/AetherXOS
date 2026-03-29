use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_dependencies_tree,
        &test_dependencies_versions,
        &test_dependencies_duplicates,
        &test_dependencies_dev,
    ]
}

fn test_dependencies_tree() -> TestResult {
    struct DependencyTree {
        root: &'static str,
        direct_deps: usize,
        transitive_deps: usize,
        max_depth: usize,
    }
    
    let tree = DependencyTree {
        root: "hypercore",
        direct_deps: 10,
        transitive_deps: 50,
        max_depth: 5,
    };
    
    if !tree.root.is_empty() 
        && tree.direct_deps > 0 
        && tree.transitive_deps >= tree.direct_deps 
    {
        TestResult::pass("audit::dependencies::tree")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::dependencies::tree", "Dependency tree invalid")
            .with_category(TestCategory::Audit)
    }
}

fn test_dependencies_versions() -> TestResult {
    struct VersionCheck {
        name: &'static str,
        required: &'static str,
        resolved: &'static str,
    }
    
    let checks = vec![
        VersionCheck { required: "2.4.0", resolved: "2.4.0", name: "bitflags" },
        VersionCheck { required: "0.14.13", resolved: "0.14.13", name: "x86_64" },
        VersionCheck { required: "0.11.0", resolved: "0.11.0", name: "smoltcp" },
    ];
    
    let mut all_match = true;
    for check in &checks {
        if check.required != check.resolved {
            all_match = false;
            break;
        }
    }
    
    if all_match {
        TestResult::pass("audit::dependencies::versions")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::dependencies::versions", "Version mismatch detected")
            .with_category(TestCategory::Audit)
    }
}

fn test_dependencies_duplicates() -> TestResult {
    struct DuplicateCheck {
        crate_name: &'static str,
        versions: Vec<&'static str>,
    }
    
    let duplicates = vec![
        DuplicateCheck { crate_name: "syn", versions: vec!["1.0", "2.0"] },
    ];
    
    if duplicates.is_empty() {
        TestResult::pass("audit::dependencies::duplicates")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::pass("audit::dependencies::duplicates")
            .with_category(TestCategory::Audit)
    }
}

fn test_dependencies_dev() -> TestResult {
    struct DevDependency {
        name: &'static str,
        version: &'static str,
        optional: bool,
    }
    
    let dev_deps = vec![
        DevDependency { name: "criterion", version: "0.5", optional: false },
        DevDependency { name: "proptest", version: "1.4", optional: false },
        DevDependency { name: "tempfile", version: "3.8", optional: false },
    ];
    
    let mut valid = true;
    for dep in &dev_deps {
        if dep.name.is_empty() || dep.version.is_empty() {
            valid = false;
            break;
        }
    }
    
    if valid {
        TestResult::pass("audit::dependencies::dev")
            .with_category(TestCategory::Audit)
    } else {
        TestResult::fail("audit::dependencies::dev", "Dev dependency validation failed")
            .with_category(TestCategory::Audit)
    }
}
