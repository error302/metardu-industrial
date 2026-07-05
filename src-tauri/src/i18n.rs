// Internationalization (i18n) — Sprint 8 Production Distribution.
//
// Translation system for English / Spanish / Portuguese.
// Spanish and Portuguese target the Latin American mining market
// (Vale, BHP, Anglo American, Codelco, Antofagasta, Grupo México).
//
// Architecture:
//   - Translation keys are string constants (e.g., "menu.file.open")
//   - Each language has a HashMap<&str, &str> of key → translation
//   - Frontend calls translate_cmd(key, lang) → returns translated string
//   - Missing translations fall back to English, then to the key itself
//
// Adding a new language:
//   1. Add it to the Language enum
//   2. Add a translation table via get_translations()
//   3. Add it to the language picker in Settings
//
// Phase 9+ will load translations from JSON files for easier contribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    En,
    Es,
    Pt,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Es => "es",
            Language::Pt => "pt",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Es => "Español",
            Language::Pt => "Português",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" | "en-us" | "en-gb" => Some(Language::En),
            "es" | "es-es" | "es-mx" | "es-ar" | "es-cl" | "es-br" => Some(Language::Es),
            "pt" | "pt-br" | "pt-pt" => Some(Language::Pt),
            _ => None,
        }
    }
}

/// Translate a key to the given language. Falls back to English if the
/// key isn't translated in the target language, then to the key itself
/// if not found in English either.
pub fn translate(key: &str, lang: Language) -> String {
    if lang != Language::En {
        if let Some(t) = get_translations(lang).get(key) {
            return (*t).to_string();
        }
    }
    // Fall back to English
    if let Some(t) = get_translations(Language::En).get(key) {
        return (*t).to_string();
    }
    // Fall back to the key itself
    key.to_string()
}

/// Get all available languages
pub fn available_languages() -> Vec<Language> {
    vec![Language::En, Language::Es, Language::Pt]
}

/// Get the translation table for a language
fn get_translations(lang: Language) -> &'static HashMap<&'static str, &'static str> {
    use std::sync::OnceLock;
    static EN: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static ES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    static PT: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

    match lang {
        Language::En => EN.get_or_init(|| {
            let mut m = HashMap::new();
            // App
            m.insert("app.name", "MetaRDU Industrial");
            m.insert("app.ready", "Ready");
            // Menu
            m.insert("menu.file", "File");
            m.insert("menu.file.new", "New Project");
            m.insert("menu.file.open", "Open Project");
            m.insert("menu.file.save", "Save Project");
            m.insert("menu.file.save_as", "Save Project As…");
            m.insert("menu.file.recent", "Recent Projects");
            m.insert("menu.edit", "Edit");
            m.insert("menu.view", "View");
            m.insert("menu.help", "Help");
            m.insert("menu.help.about", "About MetaRDU");
            // Sidebar
            m.insert("sidebar.project", "Project");
            m.insert("sidebar.mining", "Mining");
            m.insert("sidebar.marine", "Marine");
            m.insert("sidebar.qc_tools", "QC Tools");
            m.insert("sidebar.enterprise", "Enterprise");
            m.insert("sidebar.automation", "Automation");
            m.insert("sidebar.settings", "Settings");
            // Mining tools
            m.insert("mining.volume_calc", "Volume Calculator");
            m.insert("mining.classify_ground", "Classify Ground (CSF)");
            m.insert("mining.eom_reconciliation", "EoM Reconciliation");
            m.insert("mining.stockpile_audit", "Stockpile Audit");
            m.insert("mining.blast_report", "Blast Report");
            m.insert("mining.highwall_monitoring", "Highwall Monitoring");
            m.insert("mining.odm_pipeline", "ODM Pipeline");
            m.insert("mining.ml_classification", "ML Classification");
            m.insert("mining.4d_monitoring", "4D Monitoring");
            // Marine tools
            m.insert("marine.cube_surface", "CUBE Surface");
            m.insert("marine.cube_disambiguation", "CUBE Disambiguation");
            m.insert("marine.s44_compliance", "S-44 Compliance");
            m.insert("marine.s44_certificate", "S-44 Certificate");
            m.insert("marine.s57_export", "S-57 Export");
            m.insert("marine.svp_editor", "SVP Editor");
            m.insert("marine.vessel_config", "Vessel Configuration");
            m.insert("marine.dredge_audit", "Dredge Audit");
            m.insert("marine.cross_section", "Cross-Section Profiler");
            m.insert("marine.deliverable_package", "Deliverable Package");
            m.insert("marine.sss_waterfall", "SSS Waterfall");
            // Enterprise
            m.insert("enterprise.license_manager", "License Manager");
            m.insert("enterprise.benchmark", "Performance Benchmark");
            m.insert("enterprise.telemetry", "Telemetry & Crash");
            // Common
            m.insert("common.cancel", "Cancel");
            m.insert("common.ok", "OK");
            m.insert("common.save", "Save");
            m.insert("common.close", "Close");
            m.insert("common.run", "Run");
            m.insert("common.loading", "Loading…");
            m.insert("common.error", "Error");
            m.insert("common.success", "Success");
            m.insert("common.warning", "Warning");
            m
        }),
        Language::Es => ES.get_or_init(|| {
            let mut m = HashMap::new();
            // App
            m.insert("app.name", "MetaRDU Industrial");
            m.insert("app.ready", "Listo");
            // Menu
            m.insert("menu.file", "Archivo");
            m.insert("menu.file.new", "Nuevo Proyecto");
            m.insert("menu.file.open", "Abrir Proyecto");
            m.insert("menu.file.save", "Guardar Proyecto");
            m.insert("menu.file.save_as", "Guardar Proyecto Como…");
            m.insert("menu.file.recent", "Proyectos Recientes");
            m.insert("menu.edit", "Editar");
            m.insert("menu.view", "Ver");
            m.insert("menu.help", "Ayuda");
            m.insert("menu.help.about", "Acerca de MetaRDU");
            // Sidebar
            m.insert("sidebar.project", "Proyecto");
            m.insert("sidebar.mining", "Minería");
            m.insert("sidebar.marine", "Marino");
            m.insert("sidebar.qc_tools", "Herramientas QC");
            m.insert("sidebar.enterprise", "Empresa");
            m.insert("sidebar.automation", "Automatización");
            m.insert("sidebar.settings", "Configuración");
            // Mining tools
            m.insert("mining.volume_calc", "Calculadora de Volumen");
            m.insert("mining.classify_ground", "Clasificar Suelo (CSF)");
            m.insert("mining.eom_reconciliation", "Reconciliación Fin de Mes");
            m.insert("mining.stockpile_audit", "Auditoría de Acopios");
            m.insert("mining.blast_report", "Reporte de Voladura");
            m.insert("mining.highwall_monitoring", "Monitoreo de Talud");
            m.insert("mining.odm_pipeline", "Pipeline ODM");
            m.insert("mining.ml_classification", "Clasificación ML");
            m.insert("mining.4d_monitoring", "Monitoreo 4D");
            // Marine tools
            m.insert("marine.cube_surface", "Superficie CUBE");
            m.insert("marine.cube_disambiguation", "Desambiguación CUBE");
            m.insert("marine.s44_compliance", "Cumplimiento S-44");
            m.insert("marine.s44_certificate", "Certificado S-44");
            m.insert("marine.s57_export", "Exportar S-57");
            m.insert("marine.svp_editor", "Editor SVP");
            m.insert("marine.vessel_config", "Configuración de Buque");
            m.insert("marine.dredge_audit", "Auditoría de Dragado");
            m.insert("marine.cross_section", "Perfil Transversal");
            m.insert("marine.deliverable_package", "Paquete de Entrega");
            m.insert("marine.sss_waterfall", "Visor SSS");
            // Enterprise
            m.insert("enterprise.license_manager", "Administrador de Licencia");
            m.insert("enterprise.benchmark", "Benchmark de Rendimiento");
            m.insert("enterprise.telemetry", "Telemetría y Errores");
            // Common
            m.insert("common.cancel", "Cancelar");
            m.insert("common.ok", "Aceptar");
            m.insert("common.save", "Guardar");
            m.insert("common.close", "Cerrar");
            m.insert("common.run", "Ejecutar");
            m.insert("common.loading", "Cargando…");
            m.insert("common.error", "Error");
            m.insert("common.success", "Éxito");
            m.insert("common.warning", "Advertencia");
            m
        }),
        Language::Pt => PT.get_or_init(|| {
            let mut m = HashMap::new();
            // App
            m.insert("app.name", "MetaRDU Industrial");
            m.insert("app.ready", "Pronto");
            // Menu
            m.insert("menu.file", "Arquivo");
            m.insert("menu.file.new", "Novo Projeto");
            m.insert("menu.file.open", "Abrir Projeto");
            m.insert("menu.file.save", "Salvar Projeto");
            m.insert("menu.file.save_as", "Salvar Projeto Como…");
            m.insert("menu.file.recent", "Projetos Recentes");
            m.insert("menu.edit", "Editar");
            m.insert("menu.view", "Ver");
            m.insert("menu.help", "Ajuda");
            m.insert("menu.help.about", "Sobre MetaRDU");
            // Sidebar
            m.insert("sidebar.project", "Projeto");
            m.insert("sidebar.mining", "Mineração");
            m.insert("sidebar.marine", "Marinho");
            m.insert("sidebar.qc_tools", "Ferramentas QC");
            m.insert("sidebar.enterprise", "Empresa");
            m.insert("sidebar.automation", "Automação");
            m.insert("sidebar.settings", "Configurações");
            // Mining tools
            m.insert("mining.volume_calc", "Calculadora de Volume");
            m.insert("mining.classify_ground", "Classificar Solo (CSF)");
            m.insert("mining.eom_reconciliation", "Reconciliação Fim do Mês");
            m.insert("mining.stockpile_audit", "Auditoria de Pilhas");
            m.insert("mining.blast_report", "Relatório de Desmonte");
            m.insert("mining.highwall_monitoring", "Monitoramento de Talude");
            m.insert("mining.odm_pipeline", "Pipeline ODM");
            m.insert("mining.ml_classification", "Classificação ML");
            m.insert("mining.4d_monitoring", "Monitoramento 4D");
            // Marine tools
            m.insert("marine.cube_surface", "Superfície CUBE");
            m.insert("marine.cube_disambiguation", "Desambiguação CUBE");
            m.insert("marine.s44_compliance", "Conformidade S-44");
            m.insert("marine.s44_certificate", "Certificado S-44");
            m.insert("marine.s57_export", "Exportar S-57");
            m.insert("marine.svp_editor", "Editor SVP");
            m.insert("marine.vessel_config", "Configuração de Embarcação");
            m.insert("marine.dredge_audit", "Auditoria de Dragagem");
            m.insert("marine.cross_section", "Perfil Transversal");
            m.insert("marine.deliverable_package", "Pacote de Entrega");
            m.insert("marine.sss_waterfall", "Visualizador SSS");
            // Enterprise
            m.insert("enterprise.license_manager", "Gerenciador de Licença");
            m.insert("enterprise.benchmark", "Benchmark de Desempenho");
            m.insert("enterprise.telemetry", "Telemetria e Erros");
            // Common
            m.insert("common.cancel", "Cancelar");
            m.insert("common.ok", "OK");
            m.insert("common.save", "Salvar");
            m.insert("common.close", "Fechar");
            m.insert("common.run", "Executar");
            m.insert("common.loading", "Carregando…");
            m.insert("common.error", "Erro");
            m.insert("common.success", "Sucesso");
            m.insert("common.warning", "Aviso");
            m
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_english() {
        assert_eq!(translate("common.save", Language::En), "Save");
        assert_eq!(translate("menu.file.open", Language::En), "Open Project");
        assert_eq!(
            translate("mining.volume_calc", Language::En),
            "Volume Calculator"
        );
    }

    #[test]
    fn test_translate_spanish() {
        assert_eq!(translate("common.save", Language::Es), "Guardar");
        assert_eq!(translate("menu.file.open", Language::Es), "Abrir Proyecto");
        assert_eq!(
            translate("mining.volume_calc", Language::Es),
            "Calculadora de Volumen"
        );
    }

    #[test]
    fn test_translate_portuguese() {
        assert_eq!(translate("common.save", Language::Pt), "Salvar");
        assert_eq!(translate("menu.file.open", Language::Pt), "Abrir Projeto");
        assert_eq!(
            translate("mining.volume_calc", Language::Pt),
            "Calculadora de Volume"
        );
    }

    #[test]
    fn test_translate_fallback_to_english() {
        // A key that's only in English should fall back from Spanish
        assert_eq!(translate("app.name", Language::Es), "MetaRDU Industrial");
        assert_eq!(translate("app.name", Language::Pt), "MetaRDU Industrial");
    }

    #[test]
    fn test_translate_missing_key_returns_key() {
        assert_eq!(
            translate("nonexistent.key.xyz", Language::En),
            "nonexistent.key.xyz"
        );
        assert_eq!(
            translate("nonexistent.key.xyz", Language::Es),
            "nonexistent.key.xyz"
        );
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Some(Language::En));
        assert_eq!(Language::from_code("EN"), Some(Language::En));
        assert_eq!(Language::from_code("en-US"), Some(Language::En));
        assert_eq!(Language::from_code("es"), Some(Language::Es));
        assert_eq!(Language::from_code("es-MX"), Some(Language::Es));
        assert_eq!(Language::from_code("pt-BR"), Some(Language::Pt));
        assert_eq!(Language::from_code("xx"), None);
    }

    #[test]
    fn test_language_labels() {
        assert_eq!(Language::En.label(), "English");
        assert_eq!(Language::Es.label(), "Español");
        assert_eq!(Language::Pt.label(), "Português");
    }

    #[test]
    fn test_available_languages() {
        let langs = available_languages();
        assert_eq!(langs.len(), 3);
        assert!(langs.contains(&Language::En));
        assert!(langs.contains(&Language::Es));
        assert!(langs.contains(&Language::Pt));
    }

    #[test]
    fn test_mining_terms_translated() {
        // Critical mining terms that surveyors need in their language
        assert_eq!(
            translate("mining.highwall_monitoring", Language::Es),
            "Monitoreo de Talud"
        );
        assert_eq!(
            translate("mining.highwall_monitoring", Language::Pt),
            "Monitoramento de Talude"
        );
        assert_eq!(
            translate("mining.stockpile_audit", Language::Es),
            "Auditoría de Acopios"
        );
        assert_eq!(
            translate("mining.stockpile_audit", Language::Pt),
            "Auditoria de Pilhas"
        );
    }

    #[test]
    fn test_marine_terms_translated() {
        assert_eq!(
            translate("marine.dredge_audit", Language::Es),
            "Auditoría de Dragado"
        );
        assert_eq!(
            translate("marine.dredge_audit", Language::Pt),
            "Auditoria de Dragagem"
        );
        assert_eq!(translate("marine.svp_editor", Language::Es), "Editor SVP");
        assert_eq!(translate("marine.svp_editor", Language::Pt), "Editor SVP");
    }
}
