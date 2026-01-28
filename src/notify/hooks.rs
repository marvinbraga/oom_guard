use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub struct HookValidator;

impl HookValidator {
    /// Validates that a hook script exists and is executable
    pub fn validate_hook_script(script_path: &str) -> Result<()> {
        let path = Path::new(script_path);

        // Check if file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("Script does not exist: {}", script_path));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {}", script_path));
        }

        // Check if it's executable
        let metadata = fs::metadata(path)
            .context(format!("Failed to read metadata for: {}", script_path))?;
        let permissions = metadata.permissions();

        #[cfg(unix)]
        {
            let mode = permissions.mode();
            if mode & 0o111 == 0 {
                warn!(
                    "Script {} is not executable (permissions: {:o})",
                    script_path, mode
                );
                return Err(anyhow::anyhow!(
                    "Script is not executable: {}. Use 'chmod +x {}'",
                    script_path,
                    script_path
                ));
            }
            debug!(
                "Script {} is valid and executable (permissions: {:o})",
                script_path, mode
            );
        }

        Ok(())
    }

    /// Validates both pre-kill and post-kill scripts if they are provided
    pub fn validate_hooks(
        pre_kill_script: Option<&str>,
        post_kill_script: Option<&str>,
    ) -> Result<()> {
        if let Some(script) = pre_kill_script {
            info!("Validating pre-kill script: {}", script);
            if let Err(e) = Self::validate_hook_script(script) {
                error!("Pre-kill script validation failed: {}", e);
                return Err(e);
            }
            info!("Pre-kill script validated successfully");
        }

        if let Some(script) = post_kill_script {
            info!("Validating post-kill script: {}", script);
            if let Err(e) = Self::validate_hook_script(script) {
                error!("Post-kill script validation failed: {}", e);
                return Err(e);
            }
            info!("Post-kill script validated successfully");
        }

        Ok(())
    }
}

/// Environment variables passed to hook scripts:
/// - OOM_GUARD_PID: Process ID of the killed process
/// - OOM_GUARD_NAME: Name of the killed process
/// - OOM_GUARD_RSS: Resident Set Size in KiB
/// - OOM_GUARD_SCORE: OOM score of the process
pub struct HookEnvironment;

impl HookEnvironment {
    pub fn get_variable_names() -> Vec<&'static str> {
        vec![
            "OOM_GUARD_PID",
            "OOM_GUARD_NAME",
            "OOM_GUARD_RSS",
            "OOM_GUARD_SCORE",
        ]
    }

    pub fn describe() -> String {
        format!(
            "Hook scripts receive the following environment variables:\n\
             - OOM_GUARD_PID: Process ID of the killed process\n\
             - OOM_GUARD_NAME: Name of the killed process\n\
             - OOM_GUARD_RSS: Resident Set Size in KiB\n\
             - OOM_GUARD_SCORE: OOM score of the process"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_validate_nonexistent_script() {
        let result = HookValidator::validate_hook_script("/nonexistent/script.sh");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    #[cfg(unix)]
    fn test_validate_non_executable_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        let mut file = File::create(&script_path).unwrap();
        writeln!(file, "#!/bin/bash\necho 'test'").unwrap();

        // Don't make it executable
        let result = HookValidator::validate_hook_script(script_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not executable"));
    }

    #[test]
    #[cfg(unix)]
    fn test_validate_executable_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        let mut file = File::create(&script_path).unwrap();
        writeln!(file, "#!/bin/bash\necho 'test'").unwrap();
        drop(file);

        // Make it executable
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let result = HookValidator::validate_hook_script(script_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_hook_environment_variables() {
        let vars = HookEnvironment::get_variable_names();
        assert_eq!(vars.len(), 4);
        assert!(vars.contains(&"OOM_GUARD_PID"));
        assert!(vars.contains(&"OOM_GUARD_NAME"));
        assert!(vars.contains(&"OOM_GUARD_RSS"));
        assert!(vars.contains(&"OOM_GUARD_SCORE"));
    }

    #[test]
    fn test_hook_environment_description() {
        let desc = HookEnvironment::describe();
        assert!(desc.contains("OOM_GUARD_PID"));
        assert!(desc.contains("OOM_GUARD_NAME"));
        assert!(desc.contains("OOM_GUARD_RSS"));
        assert!(desc.contains("OOM_GUARD_SCORE"));
    }
}
