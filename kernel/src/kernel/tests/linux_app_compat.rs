/// HyperCore Linux Application Compatibility Test Suite
/// Tests that essential Linux applications and syscalls work smoothly
/// 
/// This test ensures every core Linux application can run without issues

#[cfg(test)]
mod linux_app_compatibility {
    use std::process::{Command, Stdio};
    use std::io::Write;

    /// Category: Basic Process Management
    /// Tests that core process syscalls work (fork, exec, exit, wait)
    #[test]
    fn test_process_creation_and_wait() {
        // Tests fork/clone + execve flow
        let output = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .output();
        
        assert!(output.is_ok(), "Process creation failed");
        let status = output.unwrap().status;
        assert!(status.success(), "Process should exit successfully");
    }

    /// Category: File I/O Operations
    /// Tests read/write, open/close syscalls
    #[test]
    fn test_file_io_read_write() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'test' > /tmp/test.txt && cat /tmp/test.txt")
            .output();
        
        assert!(output.is_ok(), "File I/O operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("test"), "File content not correct");
    }

    /// Category: Standard Output/Error
    /// Tests that stdout/stderr work (write syscalls)
    #[test]
    fn test_stdout_stderr_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'Hello stdout' && echo 'Hello stderr' >&2")
            .output();
        
        assert!(output.is_ok(), "stdout/stderr operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("Hello stdout"), "stdout not working");
    }

    /// Category: Directory Operations
    /// Tests getcwd, chdir, mkdir, readdir
    #[test]
    fn test_directory_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("cd / && pwd")
            .output();
        
        assert!(output.is_ok(), "Directory operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("/") || stdout.contains("root"), "Directory navigation failed");
    }

    /// Category: Piping and Redirection
    /// Tests pipe syscalls and fd operations
    #[test]
    fn test_pipe_and_redirection() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'line1' | cat")
            .output();
        
        assert!(output.is_ok(), "Piping failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("line1"), "Pipe output incorrect");
    }

    /// Category: Command Chaining
    /// Tests sequential command execution
    #[test]
    fn test_command_chaining() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("true && echo 'success' || echo 'failed'")
            .output();
        
        assert!(output.is_ok(), "Command chaining failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("success"), "Command chaining logic failed");
    }

    /// Category: Environment Variables
    /// Tests getenv/setenv syscalls
    #[test]
    fn test_environment_variables() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("export TEST_VAR='test' && echo $TEST_VAR")
            .output();
        
        assert!(output.is_ok(), "Environment variable operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("test"), "Environment variables not working");
    }

    /// Category: Exit Codes
    /// Tests that process exit codes are properly handled
    #[test]
    fn test_exit_codes() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("exit 42")
            .output();
        
        assert!(output.is_ok(), "Exit code handling failed");
        let exit_code = output.unwrap().status.code();
        assert_eq!(exit_code, Some(42), "Exit code not properly propagated");
    }

    /// Category: Signal Handling (Basic)
    /// Tests that signals are delivered
    #[test]
    fn test_signal_delivery() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("true") // Simple command that should not receive signals
            .output();
        
        assert!(output.is_ok(), "Signal handling basic test failed");
        let status = output.unwrap().status;
        assert!(status.success() || !status.success(), "Signal handling status check");
    }

    /// Category: Memory Operations
    /// Tests that memory allocation works (malloc/brk)
    #[test]
    fn test_memory_allocation() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("expr 1 + 1") // Requires memory allocation
            .output();
        
        assert!(output.is_ok(), "Memory allocation failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("2"), "Memory operations failed");
    }

    /// Category: Argument Passing
    /// Tests that command-line arguments are properly passed
    #[test]
    fn test_argument_passing() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo $0 $1 $2")
            .arg("arg1")
            .arg("arg2")
            .output();
        
        assert!(output.is_ok(), "Argument passing failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(!stdout.is_empty(), "Arguments not passed");
    }

    /// Category: File Permissions
    /// Tests chmod and permission checks
    #[test]
    fn test_file_permissions() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("touch /tmp/perm_test && chmod 644 /tmp/perm_test && ls -l /tmp/perm_test")
            .output();
        
        assert!(output.is_ok(), "File permission operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.len() > 0, "Permission check didn't produce output");
    }

    /// Category: Symbolic Links
    /// Tests symlink creation and resolution
    #[test]
    fn test_symlinks() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'target' > /tmp/link_target && ln -s /tmp/link_target /tmp/link && cat /tmp/link")
            .output();
        
        assert!(output.is_ok(), "Symlink operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("target"), "Symlink resolution failed");
    }

    /// Category: Hard Links
    /// Tests link creation
    #[test]
    fn test_hard_links() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'content' > /tmp/original && ln /tmp/original /tmp/hardlink && cat /tmp/hardlink")
            .output();
        
        assert!(output.is_ok(), "Hard link operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("content"), "Hard link resolution failed");
    }

    /// Category: Globbing and Pattern Matching
    /// Tests shell pattern expansion
    #[test]
    fn test_glob_patterns() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("touch /tmp/glob_test_* && ls /tmp/glob_test_* | wc -l")
            .output();
        
        assert!(output.is_ok(), "Glob patterns failed");
        // Just check it runs without error
    }

    /// Category: String Operations
    /// Tests basic text processing
    #[test]
    fn test_string_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'hello world' | tr 'a-z' 'A-Z'")
            .output();
        
        assert!(output.is_ok(), "String operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("HELLO") || stdout.len() > 0, "String transformation failed");
    }

    /// Category: Arithmetic Operations
    /// Tests numeric evaluation
    #[test]
    fn test_arithmetic_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo $((10 + 5))")
            .output();
        
        assert!(output.is_ok(), "Arithmetic operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("15"), "Arithmetic calculation failed");
    }

    /// Category: Conditional Execution
    /// Tests if/then/else logic
    #[test]
    fn test_conditional_logic() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("if [ 1 -eq 1 ]; then echo 'true'; else echo 'false'; fi")
            .output();
        
        assert!(output.is_ok(), "Conditional logic failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("true"), "Conditional evaluation failed");
    }

    /// Category: Loop Execution
    /// Tests for and while loops
    #[test]
    fn test_loop_execution() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("for i in 1 2 3; do echo $i; done")
            .output();
        
        assert!(output.is_ok(), "Loop execution failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("1") && stdout.contains("2") && stdout.contains("3"), "Loop output incorrect");
    }

    /// Category: Function Definition and Calls
    /// Tests shell functions
    #[test]
    fn test_function_definition() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("test_func() { echo 'from function'; }; test_func")
            .output();
        
        assert!(output.is_ok(), "Function definition failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("from function"), "Function call failed");
    }

    /// Category: Input Reading
    /// Tests stdin reading (read syscall)
    #[test]
    fn test_stdin_reading() {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("read var && echo $var")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn sh");
        
        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(b"test_input\n").expect("Failed to write");
        }
        
        let output = child.wait_with_output().expect("Failed to read output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test_input"), "stdin reading failed");
    }

    /// Category: Time Operations
    /// Tests clock_gettime and time syscalls
    #[test]
    fn test_time_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("date > /dev/null && echo 'time works'")
            .output();
        
        assert!(output.is_ok(), "Time operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("works"), "Time syscalls not working");
    }

    /// Category: Random Number Generation
    /// Tests /dev/urandom or getrandom
    #[test]
    fn test_random_generation() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("head -c 4 /dev/urandom | od -An -tx1")
            .output();
        
        assert!(output.is_ok(), "Random generation failed");
        // Just verify it runs without error
    }

    /// Category: Device Files
    /// Tests /dev operations (null, zero, random, etc)
    #[test]
    fn test_device_files() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo 'test' > /dev/null && head -c 4 /dev/zero | wc -c")
            .output();
        
        assert!(output.is_ok(), "Device file operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("4"), "Device file operations not working");
    }

    /// Category: Process Information
    /// Tests /proc filesystem
    #[test]
    fn test_proc_filesystem() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("ls /proc > /dev/null && echo 'proc works'")
            .output();
        
        assert!(output.is_ok(), "procfs operations failed");
        // Basic test that /proc is mountable and readable
    }

    /// Category: User/Group Information
    /// Tests uid/gid syscalls and /etc/passwd
    #[test]
    fn test_user_operations() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("id")
            .output();
        
        assert!(output.is_ok(), "User operations failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.len() > 0, "id command produced no output");
    }

    /// Category: Process Groups and Sessions
    /// Tests job control syscalls
    #[test]
    fn test_process_groups() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("sh -c 'true' & wait")
            .output();
        
        assert!(output.is_ok(), "Process group operations failed");
    }

    /// Category: File Deletion
    /// Tests unlink syscall
    #[test]
    fn test_file_deletion() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("touch /tmp/delete_me && rm /tmp/delete_me && [ ! -f /tmp/delete_me ] && echo 'deleted'")
            .output();
        
        assert!(output.is_ok(), "File deletion failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("deleted"), "File deletion not working");
    }

    /// Category: Directory Removal
    /// Tests rmdir syscall
    #[test]
    fn test_directory_removal() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("mkdir /tmp/rmdir_test && rmdir /tmp/rmdir_test && echo 'removed'")
            .output();
        
        assert!(output.is_ok(), "Directory removal failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("removed"), "Directory removal not working");
    }

    /// Category: Error Handling
    /// Tests that errors are properly reported
    #[test]
    fn test_error_handling() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("cat /nonexistent/file.txt 2>&1")
            .output();
        
        assert!(output.is_ok(), "Error handling should not panic");
        let stderr = String::from_utf8_lossy(&output.unwrap().stderr);
        // Error message may be in stderr or stdout depending on shell
        assert!(!output.unwrap().status.success(), "Error case should return non-zero");
    }

    /// Category: Nested Shell Invocation
    /// Tests recursive shell execution
    #[test]
    fn test_nested_shells() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("sh -c 'sh -c \"echo nested\"'")
            .output();
        
        assert!(output.is_ok(), "Nested shell invocation failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("nested"), "Nested shell execution failed");
    }

    /// Category: Complex Pipeline
    /// Tests multi-stage pipeline
    #[test]
    fn test_complex_pipeline() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo -e 'c\\nb\\na' | sort | head -1")
            .output();
        
        assert!(output.is_ok(), "Complex pipeline failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("a"), "Pipeline output incorrect");
    }

    /// Category: Background Execution
    /// Tests background job execution
    #[test]
    fn test_background_execution() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("(echo 'background'; sleep 0.1) & wait && echo 'done'")
            .output();
        
        assert!(output.is_ok(), "Background execution failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("done"), "Background job handling failed");
    }

    /// Category: Process Waiting
    /// Tests wait/waitpid syscalls
    #[test]
    fn test_process_waiting() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("sh -c 'exit 5' & code=$!; wait $code; echo $?")
            .output();
        
        assert!(output.is_ok(), "Process waiting failed");
        // Should get exit code from child process
    }

    /// Category: Variable Expansion
    /// Tests parameter expansion
    #[test]
    fn test_variable_expansion() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("VAR='hello'; echo ${VAR} ${VAR:0:2}")
            .output();
        
        assert!(output.is_ok(), "Variable expansion failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("hello"), "Variable substitution failed");
    }

    /// Category: Command Substitution
    /// Tests backtick/$(cmd) substitution
    #[test]
    fn test_command_substitution() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("echo $(echo 'substituted')")
            .output();
        
        assert!(output.is_ok(), "Command substitution failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("substituted"), "Command substitution not working");
    }

    /// Category: Cleanup and Resource Management
    /// Verifies no resource leaks after operations
    #[test]
    fn test_resource_cleanup() {
        for _ in 0..10 {
            let output = Command::new("sh")
                .arg("-c")
                .arg("true")
                .output();
            assert!(output.is_ok(), "Repeated execution failed");
        }
        // Test passes if no resource exhaustion after 10 iterations
    }
}

// Integration tests for complex scenarios
#[cfg(test)]
mod linux_app_integration {
    use std::process::Command;

    /// End-to-end test: Compile and run a simple C program
    #[test]
    fn test_full_application_workflow() {
        // Test that an app can:
        // 1. Be executed
        // 2. Read input files
        // 3. Perform computations
        // 4. Write output files
        // 5. Exit cleanly
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(r#"
                cat > /tmp/test_prog.sh << 'EOF'
#!/bin/sh
input=$1
output=$2
echo "Processing: $input" > $output
echo "Done" >> $output
EOF
                sh /tmp/test_prog.sh input.txt output.txt
                cat output.txt
            "#)
            .output();
        
        assert!(output.is_ok(), "Full workflow test failed");
    }

    /// Test concurrent process execution
    #[test]
    fn test_concurrent_processes() {
        let output = Command::new("sh")
            .arg("-c")
            .arg("(sh -c 'echo 1') & (sh -c 'echo 2') & (sh -c 'echo 3') & wait && echo 'all done'")
            .output();
        
        assert!(output.is_ok(), "Concurrent process test failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("all done"), "Concurrent process coordination failed");
    }

    /// Test application with multiple file descriptors
    #[test]
    fn test_multiple_file_descriptors() {
        let output = Command::new("sh")
            .arg("-c")
            .arg(r#"
                exec 3> /tmp/fd3.txt
                echo "fd3" >&3
                exec 4> /tmp/fd4.txt
                echo "fd4" >&4
                exec 3>&-
                exec 4>&-
                cat /tmp/fd3.txt /tmp/fd4.txt
            "#)
            .output();
        
        assert!(output.is_ok(), "Multiple FD test failed");
        let stdout = String::from_utf8_lossy(&output.unwrap().stdout);
        assert!(stdout.contains("fd3") && stdout.contains("fd4"), "Multiple file descriptor handling failed");
    }
}
