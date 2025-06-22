use std::process::Command;

/// Enhanced command execution with better error handling
#[allow(dead_code)]
pub fn execute_command_safe(cmd: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(format!("Success:\n{}", stdout))
    } else {
        Err(format!("Error:\n{}\nStderr: {}", stdout, stderr))
    }
}

/// Safe script execution with environment setup
#[allow(dead_code)]
pub fn execute_script_safe(executable: &str, args: &[String]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .env("DEBIAN_FRONTEND", "noninteractive") // Prevent interactive prompts
        .output()
        .map_err(|e| format!("Failed to execute script: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if output.status.success() {
        Ok(format!("Success:\n{}", stdout))
    } else {
        Err(format!("Error:\n{}\nStderr: {}", stdout, stderr))
    }
}

/// Check if we're running with appropriate privileges
#[allow(dead_code)]
pub fn check_privileges() -> bool {
    #[cfg(unix)]
    {
        let output = Command::new("id")
            .arg("-u")
            .output();
        
        if let Ok(output) = output {
            let uid_str = String::from_utf8_lossy(&output.stdout);
            let uid: u32 = uid_str.trim().parse().unwrap_or(1000);
            uid == 0 // Return true if running as root
        } else {
            false
        }
    }
    
    #[cfg(not(unix))]
    {
        true // On non-Unix systems, assume we have privileges
    }
}