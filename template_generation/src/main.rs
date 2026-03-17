use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Liquid input file name
    #[arg(long, default_value = "template_generation/liquid_prompt_config.yaml")]
    liquid_in_file: String,
    /// Template templates directory for liquid
    #[arg(long, default_value = "template_generation/templates/liquid")]
    liquid_templates_dir: String,
    /// Jinja input file name
    #[arg(long, default_value = "template_generation/jinja_prompt_config.yaml")]
    jinja_in_file: String,
    /// Template templates directory for jinja
    #[arg(long, default_value = "template_generation/templates/jinja")]
    jinja_templates_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LiquidTemplateConfig {
    RawTemplate(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        variables: HashMap<String, LiquidTemplateConfig>,
    },
}

impl LiquidTemplateConfig {
    fn get_template_string(&self, base_dir: &String) -> Result<String> {
        match self {
            LiquidTemplateConfig::RawTemplate(s) => Ok(s.clone()),
            LiquidTemplateConfig::Config { path, variables } => {
                let path_str = path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Template path is required but not provided"))?;
                let full_path = format!("{}/{}", base_dir, path_str);
                let content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {:?}", full_path))?;

                // Process variables
                if !variables.is_empty() {
                    let mut variable_strings = Vec::new();
                    let mut keys: Vec<_> = variables.keys().collect();
                    keys.sort(); // Sort for consistent output

                    for key in keys {
                        let var_content = variables[key].get_template_string(base_dir)?;
                        let var_content_trimmed = var_content.trim_end_matches('\n');
                        variable_strings.push(format!(
                            "{{% capture {} %}}{}{{% endcapture %}}",
                            key, var_content_trimmed
                        ));
                    }

                    let variables_string = variable_strings.join("");
                    Ok(format!("{}{}", variables_string, content))
                } else {
                    Ok(content)
                }
            }
        }
    }
}

fn parse_and_render_liquid(args: &Args) -> Result<()> {
    let liquid_in_file = &args.liquid_in_file;
    let liquid_templates_dir = &args.liquid_templates_dir;

    // Read the input config file
    let input_content = fs::read_to_string(&liquid_in_file)
        .with_context(|| format!("Failed to read input file: {:?}", liquid_in_file))?;

    let config: HashMap<String, LiquidTemplateConfig> = serde_yaml::from_str(&input_content)
        .with_context(|| format!("Failed to parse YAML from: {:?}", liquid_in_file))?;

    for (key, template_config) in &config {
        let template_string = template_config
            .get_template_string(liquid_templates_dir)
            .with_context(|| format!("Failed to process liquid template for key: {}", key))?;
        let out_file_path = format!("gen/templates/liquid/{}.tmpl", key);

        fs::write(&out_file_path, template_string)
            .with_context(|| format!("Failed to write output file: {:?}", out_file_path))?;
    }

    println!(
        "Successfully generated {} liquid templates from {}",
        config.len(),
        args.liquid_in_file
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JinjaTemplateConfig {
    RawTemplate(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        includes: HashMap<String, JinjaTemplateConfig>,
    },
}

impl JinjaTemplateConfig {
    fn get_template_string(&self, base_dir: &String) -> Result<String> {
        match self {
            JinjaTemplateConfig::RawTemplate(s) => Ok(s.clone()),
            JinjaTemplateConfig::Config { path, includes } => {
                let path_str = path
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Template path is required but not provided"))?;
                let full_path = format!("{}/{}", base_dir, path_str);
                let mut content = fs::read_to_string(&full_path)
                    .with_context(|| format!("Failed to read file: {:?}", full_path))?;

                // Process variables
                if !includes.is_empty() {
                    for (key, include_config) in includes {
                        let include_content = include_config
                            .get_template_string(base_dir)?
                            .trim_end_matches('\n')
                            .to_string();
                        // regex replace includes like `{% include "chat_merged_template.jinja" %}` with the include_content
                        let include_statement = format!(r#"\{{%\s*include\s*"{}"\s*%\}}"#, key);
                        let re = regex::Regex::new(&include_statement).with_context(|| {
                            format!("Failed to create regex for include: {}", key)
                        })?;
                        content = re
                            .replace_all(&content, regex::NoExpand(include_content.as_str()))
                            .to_string();

                        // Also support `{%-` which trims whitespace
                        let include_statement_trim =
                            format!(r#"\s*\{{%-\s*include\s*"{}"\s*%\}}\s*"#, key);
                        let re = regex::Regex::new(&include_statement_trim).with_context(|| {
                            format!("Failed to create regex for include: {}", key)
                        })?;
                        content = re
                            .replace_all(&content, regex::NoExpand(include_content.as_str()))
                            .to_string();
                    }
                    Ok(content)
                } else {
                    Ok(content)
                }
            }
        }
    }
}

fn parse_and_render_jinja(args: &Args) -> Result<()> {
    let jinja_in_file = &args.jinja_in_file;
    let jinja_templates_dir = &args.jinja_templates_dir;

    let input_content = fs::read_to_string(&jinja_in_file)
        .with_context(|| format!("Failed to read input file: {:?}", jinja_in_file))?;

    let config: HashMap<String, JinjaTemplateConfig> = serde_yaml::from_str(&input_content)
        .with_context(|| format!("Failed to parse YAML from: {:?}", jinja_in_file))?;

    for (key, template_config) in &config {
        let template_string = template_config
            .get_template_string(jinja_templates_dir)
            .with_context(|| format!("Failed to process jinja template for key: {}", key))?;

        let out_file_path = format!("gen/templates/jinja/{}.jinja", key);

        fs::write(&out_file_path, template_string)
            .with_context(|| format!("Failed to write output file: {:?}", out_file_path))?;
    }

    println!(
        "Successfully generated {} jinja templates from {}",
        config.len(),
        args.jinja_in_file
    );

    Ok(())
}

fn main() {
    let args = Args::parse();

    if let Err(e) = parse_and_render_liquid(&args) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }

    if let Err(e) = parse_and_render_jinja(&args) {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
