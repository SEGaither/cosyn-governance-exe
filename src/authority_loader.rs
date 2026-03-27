pub const CGS_RAW: &str = include_str!("../governance/artifacts/cosyn-constitution-v15.1.0.md");
pub const GOVERNOR_RAW: &str = include_str!("../governance/artifacts/Persona_Governor_v2.4.2.md");
pub const ARCHITECT_RAW: &str = include_str!("../governance/artifacts/Stack_Architect_v2.3.2.md");

pub struct AuthorityBundle {
    pub cgs_raw: &'static str,
    pub governor_raw: &'static str,
    pub architect_raw: &'static str,
}

pub fn load_embedded_authorities() -> AuthorityBundle {
    AuthorityBundle {
        cgs_raw: CGS_RAW,
        governor_raw: GOVERNOR_RAW,
        architect_raw: ARCHITECT_RAW,
    }
}

pub fn validate_authorities(bundle: &AuthorityBundle) -> Result<(), &'static str> {
    if !bundle.cgs_raw.contains("CoSyn Constitution v15.1.0") {
        return Err("CGS identity/version invalid");
    }

    if !bundle.governor_raw.contains("Persona Governor v2.4.2") {
        return Err("Governor version invalid");
    }

    if !bundle.architect_raw.contains("Stack Architect v2.3.2") {
        return Err("Architect version invalid");
    }

    Ok(())
}