use crate::harness::{TestResult, TestCategory};

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_unused_variables,
        &test_unused_imports,
        &test_unused_functions,
        &test_dead_code,
    ]
}

fn test_unused_variables() -> TestResult {
    let used_variable = 42;
    let _unused_variable = 100;
    
    let result = used_variable * 2;
    
    if result == 84 {
        TestResult::pass("lint::unused::variables")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::unused::variables", "Unused variable test failed")
            .with_category(TestCategory::Lint)
    }
}

fn test_unused_imports() -> TestResult {
    use core::fmt::Debug;
    
    struct TestStruct {
        value: i32,
    }
    
    impl Debug for TestStruct {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "TestStruct {{ value: {} }}", self.value)
        }
    }
    
    let test = TestStruct { value: 42 };
    let debug_str = format!("{:?}", test);
    
    if debug_str.contains("42") {
        TestResult::pass("lint::unused::imports")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::unused::imports", "Unused import test failed")
            .with_category(TestCategory::Lint)
    }
}

fn test_unused_functions() -> TestResult {
    #[allow(dead_code)]
    fn unused_function() -> i32 {
        42
    }
    
    fn used_function() -> i32 {
        100
    }
    
    let result = used_function();
    
    if result == 100 {
        TestResult::pass("lint::unused::functions")
            .with_category(TestCategory::Lint)
    } else {
        TestResult::fail("lint::unused::functions", "Unused function test failed")
            .with_category(TestCategory::Lint)
    }
}

fn test_dead_code() -> TestResult {
    enum TestEnum {
        #[allow(dead_code)]
        Unused,
        Used,
    }
    
    let value = TestEnum::Used;
    
    match value {
        TestEnum::Used => {
            TestResult::pass("lint::unused::dead_code")
                .with_category(TestCategory::Lint)
        }
        TestEnum::Unused => {
            TestResult::fail("lint::unused::dead_code", "Dead code reached")
                .with_category(TestCategory::Lint)
        }
    }
}
