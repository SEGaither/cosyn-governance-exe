use std::fs;
use std::path::Path;

pub const CGS_RAW: &str = include_str!("../governance/artifacts/cosyn-constitution-v15.1.0.md");
pub const GOVERNOR_RAW: &str = include_str!("../governance/artifacts/Persona_Governor_v2.4.2.md");
pub const ARCHITECT_RAW: &str = include_str!("../governance/artifacts/Stack_Architect_v2.3.2.md");

pub struct AuthorityBundle {
    pub cgs_raw: String,
    pub governor_raw: String,
    pub architect_raw: String,
}

pub fn load_embedded_authorities() -> AuthorityBundle {
    AuthorityBundle {
        cgs_raw: CGS_RAW.to_string(),
        governor_raw: GOVERNOR_RAW.to_string(),
        architect_raw: ARCHITECT_RAW.to_string(),
    }
}

pub fn load_authorities_from_dir(dir: &Path) -> Result<AuthorityBundle, String> {
    let cgs = read_artifact(dir, "cgs")?;
    let governor = read_artifact(dir, "governor")?;
    let architect = read_artifact(dir, "architect")?;

    Ok(AuthorityBundle {
        cgs_raw: cgs,
        governor_raw: governor,
        architect_raw: architect,
    })
}

fn read_artifact(dir: &Path, role: &str) -> Result<String, String> {
    let pattern = match role {
        "cgs" => "cosyn-constitution",
        "governor" => "Persona_Governor",
        "architect" => "Stack_Architect",
        _ => return Err(format!("Unknown artifact role: {}", role)),
    };

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.contains(pattern) && name_str.ends_with(".md") {
            let content = fs::read_to_string(entry.path())
                .map_err(|e| format!("Cannot read {}: {}", entry.path().display(), e))?;
            return Ok(content);
        }
    }

    Err(format!("No {} artifact found in {}", role, dir.display()))
}

pub fn validate_authorities(bundle: &AuthorityBundle) -> Result<(), String> {
    if !bundle.cgs_raw.contains("CoSyn Constitution") {
        return Err("CGS identity invalid".to_string());
    }

    if !bundle.governor_raw.contains("Persona Governor") {
        return Err("Governor identity invalid".to_string());
    }

    if !bundle.architect_raw.contains("Stack Architect") {
        return Err("Architect identity invalid".to_string());
    }

    Ok(())
}
