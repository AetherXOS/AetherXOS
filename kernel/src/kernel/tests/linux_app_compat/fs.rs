use std::process::Command;

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

#[test]
fn test_glob_patterns() {
    let output = Command::new("sh")
        .arg("-c")
        .arg("touch /tmp/glob_test_* && ls /tmp/glob_test_* | wc -l")
        .output();
    
    assert!(output.is_ok(), "Glob patterns failed");
}

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

#[test]
fn test_proc_filesystem() {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ls /proc > /dev/null && echo 'proc works'")
        .output();
    
    assert!(output.is_ok(), "procfs operations failed");
}

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
