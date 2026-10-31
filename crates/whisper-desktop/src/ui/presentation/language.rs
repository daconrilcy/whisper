use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageChoices {
    pub auto_language_selectable: bool,
    pub auto_language_cause: Option<whisper_core::engine_capabilities::RefusalCause>,
    pub languages: Vec<whisper_core::engine_capabilities::LanguageOption>,
}

/// Projects language capability for the selected backend. Auto only exposes choices supported
/// by both possible backends because its effective backend can be either one.
pub fn language_choices_for_compute(
    model: &whisper_core::engine_capabilities::ModelOption,
    compute: ComputeChoice,
) -> Option<LanguageChoices> {
    use whisper_core::engine_capabilities::{LanguageOption, RefusalCause};
    let find = |backend| {
        model
            .configurations
            .iter()
            .find(|config| config.key.backend == backend)
    };
    let unavailable = |config_cause: Option<RefusalCause>| {
        config_cause.or(Some(RefusalCause::ConfigurationUnsupported))
    };
    match compute {
        ComputeChoice::Cpu | ComputeChoice::Gpu => {
            let backend = if compute == ComputeChoice::Cpu {
                Backend::Cpu
            } else {
                Backend::Gpu
            };
            let config = find(backend)?;
            Some(LanguageChoices {
                auto_language_selectable: config.selectable && config.auto_language_selectable,
                auto_language_cause: if config.selectable {
                    config.auto_language_cause
                } else {
                    unavailable(config.cause)
                },
                languages: config
                    .languages
                    .iter()
                    .map(|language| LanguageOption {
                        code: language.code.clone(),
                        selectable: config.selectable && language.selectable,
                        cause: if config.selectable {
                            language.cause
                        } else {
                            unavailable(config.cause)
                        },
                    })
                    .collect(),
            })
        }
        ComputeChoice::Auto => {
            let cpu = find(Backend::Cpu)?;
            let gpu = find(Backend::Gpu)?;
            Some(LanguageChoices {
                auto_language_selectable: cpu.selectable
                    && gpu.selectable
                    && cpu.auto_language_selectable
                    && gpu.auto_language_selectable,
                auto_language_cause: if !cpu.selectable || !gpu.selectable {
                    unavailable(cpu.cause.or(gpu.cause))
                } else {
                    cpu.auto_language_cause.or(gpu.auto_language_cause)
                },
                languages: cpu
                    .languages
                    .iter()
                    .map(|cpu_language| {
                        let gpu_language = gpu
                            .languages
                            .iter()
                            .find(|language| language.code == cpu_language.code);
                        let selectable = cpu.selectable
                            && gpu.selectable
                            && cpu_language.selectable
                            && gpu_language.is_some_and(|language| language.selectable);
                        let cause = if !cpu.selectable || !gpu.selectable {
                            unavailable(cpu.cause.or(gpu.cause))
                        } else if !cpu_language.selectable {
                            cpu_language
                                .cause
                                .or(Some(RefusalCause::ConfigurationUnsupported))
                        } else if let Some(gpu_language) = gpu_language {
                            gpu_language
                                .cause
                                .or(Some(RefusalCause::ConfigurationUnknown))
                        } else {
                            Some(RefusalCause::ConfigurationUnknown)
                        };
                        LanguageOption {
                            code: cpu_language.code.clone(),
                            selectable,
                            cause: if selectable { None } else { cause },
                        }
                    })
                    .collect(),
            })
        }
    }
}

pub(in crate::ui) fn language_choice_labels(
    choices: &LanguageChoices,
) -> Vec<(&whisper_core::engine_capabilities::LanguageOption, String)> {
    choices
        .languages
        .iter()
        .map(|language| {
            let label = TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(code, _)| *code == language.code.0)
                .map(|(code, name)| format!("{name} ({code})"))
                .unwrap_or_else(|| language.code.0.clone());
            (language, label)
        })
        .collect()
}

pub fn language_option_labels(
    configuration: &whisper_core::engine_capabilities::ConfigurationOption,
) -> Vec<(&whisper_core::engine_capabilities::LanguageOption, String)> {
    configuration
        .languages
        .iter()
        .map(|language| {
            let code = language.code.0.as_str();
            let name = TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(known, _)| *known == code)
                .map_or(code, |(_, english)| *english);
            (language, format!("{name} ({code})"))
        })
        .collect()
}
