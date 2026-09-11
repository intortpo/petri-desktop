use std::path::PathBuf;
use std::process::Command;

pub fn discover_skills(project_path: &str) -> Vec<String> {
    let mut skills = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    
    let dirs_to_scan = vec![
        PathBuf::from(&home).join(".gemini").join("config").join("skills"),
        PathBuf::from(project_path).join(".agents").join("skills"),
        PathBuf::from(&home).join(".mesh").join("skills"),
    ];
    
    for dir in dirs_to_scan {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Some(name) = entry.file_name().to_str() {
                                skills.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    skills.sort();
    skills.dedup();
    skills
}

pub fn install_plugin(url: &str) -> Result<String, String> {
    let output = Command::new("agy")
        .arg("plugin")
        .arg("install")
        .arg(url)
        .output();
        
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        },
        Err(e) => Err(format!("Failed to execute agy plugin install: {}", e))
    }
}
